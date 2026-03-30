# Audit: WP-1 Workflow Projection Correlation v1 Smoketest Proof Run Review

## METADATA

- AUDIT_ID: AUDIT-20260329-WORKFLOW-PROJECTION-CORRELATION-V1-SMOKETEST-PROOF-RUN-REVIEW
- SMOKETEST_REVIEW_ID: SMOKETEST-REVIEW-20260329-WORKFLOW-PROJECTION-CORRELATION-V1
- REVIEW_KIND: PROOF_RUN
- DATE_UTC: 2026-03-29
- AUTHOR: Codex acting as Orchestrator
- HISTORICAL_BASELINE_PACKET: NONE
- ACTIVE_RECOVERY_PACKET: WP-1-Workflow-Projection-Correlation-v1
- LINEAGE_STATUS: LIVE_SMOKETEST_BASELINE_PENDING
- RELATED_PREVIOUS_REVIEWS:
  - NONE
- SCOPE:
  - first formal smoketest review for `WP-1-Workflow-Projection-Correlation-v1`
  - signed six-file packet scope for workflow-run and workflow-node correlation export
  - orchestrator-managed ACP lane using dedicated coder, WP validator, and integration-validator sessions
  - packet, receipts, runtime status, validator-gate evidence, clean-room patch artifact, and current local `main` baseline at `a1fb1773e5cf506ec9d926a14ce7b0c0d2bf025c`
- RESULT:
  - PRODUCT_REMEDIATION: PASS
  - MASTER_SPEC_AUDIT: PASS
  - WORKFLOW_DISCIPLINE: PARTIAL
  - ACP_RUNTIME_DISCIPLINE: PARTIAL
  - MERGE_PROGRESSION: FAIL
- KEY_COMMITS_REVIEWED:
  - `2743411` `feat: finalize WP-1 workflow projection correlation bundle export`
  - `a1fb177` `gov: sync governance kernel 881c4b6`
- EVIDENCE_SOURCES:
  - `.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md`
  - `.GOV/roles_shared/docs/POST_SMOKETEST_IMPROVEMENT_RUBRIC.md`
  - `.GOV/task_packets/WP-1-Workflow-Projection-Correlation-v1/packet.md`
  - `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Workflow-Projection-Correlation-v1/RUNTIME_STATUS.json`
  - `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Workflow-Projection-Correlation-v1/RECEIPTS.jsonl`
  - `../gov_runtime/roles_shared/validator_gates/WP-1-Workflow-Projection-Correlation-v1.json`
  - `../gov_runtime/roles_shared/ROLE_SESSION_REGISTRY.json`
  - `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Workflow-Projection-Correlation-v1/isolated-six-file.patch`
  - `../wtc-projection-correlation-v1/src/backend/handshake_core/src/bundles/exporter.rs`
  - `../wtc-projection-correlation-v1/src/backend/handshake_core/src/bundles/schemas.rs`
  - `../wtc-projection-correlation-v1/src/backend/handshake_core/src/bundles/templates.rs`
  - `../wtc-projection-correlation-v1/src/backend/handshake_core/src/bundles/validator.rs`
  - `../wtc-projection-correlation-v1/src/backend/handshake_core/src/bundles/zip.rs`
  - `../wtc-projection-correlation-v1/src/backend/handshake_core/src/workflows.rs`
- RELATED_GOVERNANCE_ITEMS:
  - `RGF-17` in `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `RGF-18` in `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `RGF-19` in `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `RGF-20` in `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `RGF-21` in `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `RGF-22` in `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `RGF-28` in `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `RGF-29` in `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `RGF-30` in `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `RGF-31` in `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- RELATED_CHANGESETS:
  - signed six-file product delta in `bundles/exporter.rs`, `bundles/schemas.rs`, `bundles/templates.rs`, `bundles/validator.rs`, `bundles/zip.rs`, and `workflows.rs`

---

## 1. Executive Summary

- The signed product scope is technically strong. The workflow-run and workflow-node correlation export surfaces were implemented inside the signed six-file boundary, the clean-room patch artifact still anchors the technical proof, and the direct coder <-> validator lane is now genuinely doing attack-style review rather than status theater.
- The run is still not governance-closed. The backup branch is now current, the direct-review lane is complete, but the live coder checkout again fails committed-handoff gates after the unrelated dirty files were restored. Current-main compatibility and merge progression are therefore still blocked, and the packet/runtime/task-board truth surfaces remain stale relative to what has and has not actually been proven.

## 2. Lineage and What This Run Needed To Prove

- This is the first formal smoketest review for `WP-1-Workflow-Projection-Correlation-v1`.
- The run needed to prove four truths:
  - the signed packet could add first-class `workflow_run` and `workflow_node_execution` bundle scope support without widening into API or UI surfaces
  - the correlated exporter, manifest schema, validator, and inventory surfaces would hold together on clean current `main`
  - the WP validator would actively steer bootstrap, bounded reset, and technical blocker discovery instead of only reviewing a late coder claim
  - the orchestrator-managed ACP lane could carry that proof into committed-handoff and final-review progression without operator relay

### What Improved vs Previous Smoketest

- NONE as a formal review lineage, because there is no earlier smoketest review for this packet.
- Inside this run itself, several real improvements landed after the first draft state:
  - the remote backup branch now points at the approved WP commit `274341181b694e8ae6699b047117d136bbd3f041`
  - the direct-review lane is now fully green with `open_review_items=0`
  - the repo proved that committed-handoff can pass when unrelated out-of-scope dirt is parked and the signed commit is evaluated cleanly
- What did not improve enough:
  - that committed-handoff PASS did not remain sticky after the unrelated dirt was restored
  - final integration closeout is still blocked because packet compatibility fields and runtime/task-board projections still lag the real proof chain

## 3. Product Outcome

- Product code changed only in the signed six-file surface:
  - `src/backend/handshake_core/src/bundles/exporter.rs`
  - `src/backend/handshake_core/src/bundles/schemas.rs`
  - `src/backend/handshake_core/src/bundles/templates.rs`
  - `src/backend/handshake_core/src/bundles/validator.rs`
  - `src/backend/handshake_core/src/bundles/zip.rs`
  - `src/backend/handshake_core/src/workflows.rs`
- The clean-room artifact `isolated-six-file.patch` remains the strongest proof asset. It demonstrates that the signed scope can be applied against clean `handshake_main` and that the named packet tests hold there.
- On current evidence, the signed scope is closed in substance against the Master Spec clauses named in the packet.
- Remaining debt is not a live product/spec gap inside the packet. Remaining debt is workflow closeout truth:
  - the current live coder checkout still carries unrelated dirty product files outside signed scope
  - `CURRENT_MAIN_COMPATIBILITY_STATUS` is still `NOT_RUN` in the packet
  - merge progression is not yet recorded

## 4. Timeline

- `2026-03-29T00:42Z`: WP validator kickoff established the bounded workflow correlation scope and first proof target.
- `2026-03-29T01:08Z`: orchestrator recorded a scope invalidity after the coder widened into `api/bundles.rs`.
- `2026-03-29T01:12Z` to `2026-03-29T01:29Z`: WP validator forced the bounded reset, scope repair, and early microtask steering.
- `2026-03-29T02:45Z`: WP validator found the clean-room compile-contract blocker in `workflows.rs` against clean current `main`.
- `2026-03-29T02:56Z` to `2026-03-29T03:03Z`: coder repaired the compile-contract drift; WP validator reran the clean-room proof and accepted it.
- `2026-03-29T03:18Z` to `2026-03-29T03:47Z`: integration review opened; packet hygiene and direct-review closure were repaired until `open_review_items=0`.
- `2026-03-29T06:36Z`: committed-handoff evidence was temporarily forced green by pushing the backup branch, parking unrelated dirt, rerunning `pre-work` and `post-work`, and then restoring the parked dirt.
- Current state: technical proof remains strong, but the live lane is still blocked because the restored coder checkout again fails committed-handoff and the packet/runtime projections were not advanced to match the temporary proof window.

## 5. Failure Inventory

### 5.1 High: Committed-Handoff PASS Was Real But Not Durable

Evidence:

- `git ls-remote --heads origin feat/WP-1-Workflow-Projection-Correlation-v1` now resolves to `274341181b694e8ae6699b047117d136bbd3f041`.
- The repo demonstrated a passing committed-handoff cycle while unrelated drift was parked, but the live rerun now fails: `just validator-handoff-check WP-1-Workflow-Projection-Correlation-v1` reports `pre_work_status=FAIL` and `post_work_status=FAIL`.
- The coder worktree still carries unrelated dirty product files outside packet scope.

Reason:

- The signed WP commit is clean, but the active coder checkout is not. The workflow used a reversible stash/restore cycle to prove the commit cleanly, then restored the unrelated dirt, which returned the live checkout to a failing state.

Impact:

- Final committed-handoff truth is not sticky.
- Integration closeout cannot advance lawfully from the current live checkout.

Judgment:

- This is the main workflow blocker now. The product proof exists, but the lane still lacks a durable closeout state that survives normal worktree restoration.

### 5.2 High: Final Packet and Runtime Truth Still Lag The Proven Evidence

Evidence:

- `packet.md` still reports `CURRENT_MAIN_COMPATIBILITY_STATUS: NOT_RUN` and `PACKET_WIDENING_DECISION: NONE`.
- `RUNTIME_STATUS.json` still reports `runtime_status: input_required`, `current_phase: BOOTSTRAP`, `main_containment_status: NOT_STARTED`, and a stale heartbeat timeline.
- `TASK_BOARD.md` still shows `WP-1-Workflow-Projection-Correlation-v1` as `[IN_PROGRESS]`.
- `just integration-validator-closeout-check WP-1-Workflow-Projection-Correlation-v1` fails only because `CURRENT_MAIN_COMPATIBILITY_STATUS=COMPATIBLE` has not been recorded.

Reason:

- The lane accumulated real proof faster than the packet/runtime/task-board projection surfaces were updated.

Impact:

- The live source of truth is fragmented.
- Extra rereads and manual reconciliation are still required to know whether the remaining blocker is product, closeout, or record drift.

Judgment:

- This is classic false-green/false-red debt. Some surfaces still look too early, while others momentarily looked later than the durable live state justified.

### 5.3 Medium: Integration Validator Closeout Is Still Packet-Header Bound

Evidence:

- `just integration-validator-closeout-check WP-1-Workflow-Projection-Correlation-v1` currently fails on one remaining item: `PASS-ready closeout requires CURRENT_MAIN_COMPATIBILITY_STATUS=COMPATIBLE, not NOT_RUN`.
- The product backup branch is current, and the direct-review lane is already complete.

Reason:

- The final integration lane is appropriately strict, but its readiness still depends on packet metadata that has not been advanced to reflect the proven clean-room chain.

Impact:

- The integration validator cannot publish a clean final PASS path yet.
- Merge progression remains FAIL in this review even though the remaining blocker is narrow.

Judgment:

- This is acceptable strictness, but it exposes that packet closeout authoring is still too manual relative to the evidence already generated.

## 6. Role Review

### 6.1 Orchestrator Review

Strengths:

- Started and maintained the dedicated coder, WP validator, and integration-validator ACP lanes.
- Kept the workflow inside the expected worktree set and did not create extra worktrees after launch.
- Preserved the backup branch before the temporary stash/restore cycle.

Failures:

- Steering overhead remained high near closeout.
- The orchestrator still had to reconcile packet, runtime, gate JSON, and task-board truth manually.
- The workflow did not land in a durable green state after the temporary handoff-pass cycle.

Assessment:

- PARTIAL. The orchestrator-managed lane is materially better than earlier trials, but late-stage closeout remains too maintenance-heavy.

### 6.2 Coder Review

Strengths:

- Repaired the clean-room compile-contract mismatch in `workflows.rs` inside the signed six-file surface only.
- Produced a valid clean-room patch artifact and packet evidence strong enough for the WP validator to independently rerun.
- Stayed inside the signed product boundary after the early scope reset.

Failures:

- The first implementation attempt widened into `api/bundles.rs`.
- Packet hygiene needed late repair.
- The live coder checkout still contains unrelated dirty product files that block durable handoff truth.

Assessment:

- PARTIAL trending positive. The final code looks strong, but the live checkout and packet-hygiene discipline still cost extra validation loops.

### 6.3 WP Validator Review

Strengths:

- Performed real early steering instead of waiting for a final coder claim.
- Caught the semantic lineage issues in exporter logic, the clean-room compile-contract drift in `workflows.rs`, and the packet-hygiene blocker.
- Forced negative proof and independently reran the clean-room chain.

Failures:

- The final blocker state was still not collapsed into one durable machine-visible surface.
- The validator could prove a clean handoff under temporary parked-dirt conditions, but that did not become a lasting closeout state.

Assessment:

- PASS on technical review quality. This role is the clearest sign that the newer orchestrator-managed ACP flow is improving.

### 6.4 Integration Validator Review

Strengths:

- Opened the final review without allowing premature PASS language.
- Correctly held the line on current-main compatibility and packet-closeout requirements.

Failures:

- Final-lane progression still depends on packet-header truth that has not caught up to the actual proof chain.
- The lane has not yet published a final validation report or merge-ready disposition.

Assessment:

- PARTIAL. The authority split is right, but closeout ergonomics are still too manual.

## 7. Review Of Coder and Validator Communication

- This is the strongest part of the run.
- The direct-review lane carried real technical work:
  - kickoff and coder intent
  - bounded-reset review
  - clean-room compile blocker
  - clean-room acceptance
  - packet hygiene blocker
  - final integration review request and response
- `just wp-communication-health-check WP-1-Workflow-Projection-Correlation-v1 VERDICT` now passes with `open_review_items=0`.
- The remaining weakness is endgame compression: once technical review was already good, the lane still needed extra orchestrator mediation to translate that into durable closeout truth.

## 8. ACP Runtime / Session Control Findings

- The governed ACP sessions stayed registered and steerable for coder, WP validator, and integration validator.
- No extra role worktrees were created after launch.
- The broker/session control layer did not appear to corrupt the lane.
- Runtime truth is still stale:
  - `RUNTIME_STATUS.json` remains at `BOOTSTRAP`
  - `main_containment_status` remains `NOT_STARTED`
  - heartbeat timestamps are stale relative to the actual receipt and gate activity
- This means session control worked, but runtime projection did not stay synchronized with the real closeout phase.

## 9. Governance Implications

- Repo law around destructive or state-hiding git actions is correct and should stay strict.
- The workflow proved that backup preservation plus temporary parking can safely expose the committed WP head for validation.
- The missing piece is durable truth propagation:
  - packet metadata
  - runtime status
  - task-board projection
  - committed-handoff summary
- Without that propagation, the lane remains vulnerable to both false greens and false reds.

## 10. Positive Signals Worth Preserving

- The WP validator acted as the first technical judge for bootstrap, bounded reset, and microtask direction.
- The clean-room patch artifact gave the validator a trustworthy way to test the signed scope against clean current `main`.
- The direct-review receipt contract worked well once the packet and runtime surfaces were kept aligned.
- The backup branch is now preserved at the exact approved WP head.
- The lane stayed inside the expected ACP role/session topology with no helper-agent substitution.

## 11. Remaining Product or Spec Debt

- NONE inside the signed WP scope on current evidence.
- Remaining visible debt is workflow and closeout debt:
  - live coder checkout still fails committed-handoff because unrelated dirt was restored
  - `CURRENT_MAIN_COMPATIBILITY_STATUS` and widening/containment packet fields are stale
  - runtime/task-board surfaces still lag actual proof state
  - final integration-validator validation report has not been appended yet

## 12. Post-Smoketest Improvement Rubric

### 12.1 Workflow Smoothness

- TREND: IMPROVED
- CURRENT_STATE: MEDIUM
- Evidence:
  - the WP validator actively steered the coder through bootstrap, bounded reset, compile-contract repair, and packet hygiene
  - operator relay was not needed for the technical review loop
  - direct review is now complete and the backup branch is current
  - closeout is still repair-heavy because committed-handoff is only provable cleanly after parking unrelated dirt, and the live packet/runtime/task-board surfaces still lag the lane
- What improved:
  - direct coder <-> validator traffic is now carrying real technical review instead of shallow status chat
  - the workflow now has a credible clean-room and backup-preservation pattern
- What still hurts:
  - late-stage truth is still split across packet, runtime, gate JSON, and task-board projection
  - the committed-handoff PASS is not durable once the original unrelated dirt is restored
- Next structural fix:
  - add one canonical closeout sync step that promotes proven handoff evidence into packet/runtime/task-board truth immediately after the temporary clean-state validation succeeds

### 12.2 Master Spec Gap Reduction

- TREND: IMPROVED
- CURRENT_STATE: LOW
- Evidence:
  - the signed six-file surface now proves workflow-run and workflow-node correlation export law in clean-room against current `main`
  - the WP validator forced repair of the `workflows.rs` clean-room contract drift
  - no remaining live product/spec gap is visible inside the packet
- What improved:
  - the packet moved from speculative implementation to independently rerun clean-room proof
  - remaining uncertainty is no longer product semantics; it is closeout hygiene only
- What still hurts:
  - current-main compatibility is still not recorded in the packet even though the remaining blocker is narrow and explicit
- Next structural fix:
  - append the final integration-validator report and compatibility metadata as soon as the closeout proof window exists, so product closure and governance closure do not drift apart

### 12.3 Token Cost Pressure

- TREND: IMPROVED
- CURRENT_STATE: HIGH
- Evidence:
  - repeated orchestrator steering was still needed near closeout
  - packet-hygiene repair happened late
  - stale lifecycle projection and repeated truth-surface reconciliation caused extra rereads and duplicate checks
  - temporary clean-state validation plus restore is safer than blind closeout, but it is still expensive in prompts and manual sequencing
- What improved:
  - direct validator steering reduced the need for operator relay
  - the backup branch and clean-room proof narrowed the remaining uncertainty sharply
- What still hurts:
  - late-stage governance and projection friction still burns tokens even after the product work is basically correct
  - repeated command-surface and truth-surface checks still signal ambiguity
- Next structural fix:
  - move packet-hygiene and closeout-metadata requirements earlier, and add a governed closeout helper that snapshots the committed WP head, validates it cleanly, and writes the resulting packet/runtime progression in one path

## 13. Silent Failures, Command Surface Misuse, and Ambiguity Scan

### 13.1 Silent Failures / False Greens

- The product proof looked nearly done before the packet compatibility fields and merge-progression truth were actually settled.
- The temporary committed-handoff PASS could easily be mistaken for full closeout if the restored dirty state were not checked again.
- `RUNTIME_STATUS.json` still presents a much earlier lane phase than the real proof chain reached.

### 13.2 Systematic Wrong Tool or Command Calls

- NONE as a primary blocker in this slice. The main issue was not wrong commands; it was that the correct commands proved more than the durable packet/runtime projections recorded.

### 13.3 Task and Path Ambiguity

- The signed product diff, the clean-room artifact, the coder checkout, the integration-validator checkout, and the remote backup branch still require careful manual distinction.
- Late-stage authority truth is still split between packet status, runtime status, validator-gate JSON, and task-board projection.

### 13.4 Read Amplification / Governance Document Churn

- This run still paid for repeated receipts/runtime/gate/packet rereads because there is no single final-lane truth surface that stays current.
- Rechecking whether the handoff PASS remained true after worktree restoration was necessary and valid, but it is still evidence that closeout truth is too fragmented.

### 13.5 Hardening Direction

- Add a governed closeout-sync helper that:
  - preserves the committed backup branch
  - validates the committed WP head in a temporary clean state
  - writes packet compatibility/containment fields
  - advances runtime/task-board truth in the same step
- Make stale runtime/task-board projections a hard warning once direct-review verdict is already complete.

## 14. Suggested Remediations

### Governance / Runtime

- Add a single closeout-sync path for orchestrator-managed lanes after temporary clean-state validation succeeds.
- Make `RUNTIME_STATUS.json` and task-board projection advance automatically when committed-handoff or integration closeout proofs are recorded.
- Distinguish temporary clean-state proof from durable live checkout truth in a first-class gate field so the lane does not appear greener than it is.

### Product / Validation Quality

- Preserve the clean-room patch artifact pattern for future shared-surface packets.
- Keep the WP validator in the early steering role; this run shows that pattern is materially better than late-only review.

### Documentation / Review Practice

- Keep creating proof-run reviews when product truth is strong but closeout truth is not yet clean.
- Update reviewer guidance so packet compatibility metadata and final validation-report authoring are checked before late final-review escalation.

## 15. Command Log

- `just wp-communication-health-check WP-1-Workflow-Projection-Correlation-v1 VERDICT` -> PASS (direct review lane complete; `open_review_items=0`)
- `git ls-remote --heads origin feat/WP-1-Workflow-Projection-Correlation-v1` -> PASS (backup branch now points to `274341181b694e8ae6699b047117d136bbd3f041`)
- `just validator-handoff-check WP-1-Workflow-Projection-Correlation-v1` -> FAIL (live coder checkout still fails committed-handoff after unrelated dirt was restored)
- `just integration-validator-closeout-check WP-1-Workflow-Projection-Correlation-v1` -> FAIL (remaining blocker is `CURRENT_MAIN_COMPATIBILITY_STATUS=NOT_RUN`)

## 16. Appendage: Closeout Cost and Mechanical-Failure Evaluation

### 16.1 Why This Appendage Exists

- This appendage records the Operator-raised concern that the orchestrator-managed ACP lane is still burning too much time, too many tokens, and too much paid-model spend for work that should already be mostly mechanical.
- The goal here is not to relitigate product correctness. The product WP is now contained in `main`. The goal is to record why the repo-governance lane still behaved expensively and what must be fixed before the next orchestrator-managed trial.

### 16.2 Hard Cost Findings

- The governed WP token ledger materially under-reported the cost of this run.
  - `../gov_runtime/roles_shared/WP_TOKEN_USAGE/WP-1-Workflow-Projection-Correlation-v1.json` reports only:
    - `turn_count: 3`
    - `input_tokens: 46804987`
    - `output_tokens: 237288`
- Raw `turn.completed` evidence in `../gov_runtime/roles_shared/SESSION_CONTROL_OUTPUTS/` shows the real cost was much higher:
  - CODER: `12` turns, about `307679864` input tokens, `1492571` output tokens
  - WP_VALIDATOR: `14` turns, about `73570193` input tokens, `412545` output tokens
  - INTEGRATION_VALIDATOR: `7` turns, about `13244081` input tokens, `107110` output tokens
  - Aggregate actual cost observed from raw session outputs:
    - `turn_count: 33`
    - `input_tokens: 394494138`
    - `cached_input_tokens: 374433024`
    - `output_tokens: 2012226`
- Because the ledger is currently false, token-cost diagnostics and later budget tuning are also false.

### 16.3 Mechanical Failure Findings

- The lane is still paying for large text surfaces instead of compact machine summaries.
  - WP validator consumed a huge `git status --short --branch` dump from a dirty worktree that included large shared-`.GOV` junction drift and runtime residue.
  - WP validator also loaded a near-full packet body (`Get-Content ...packet.md -TotalCount 760`) instead of a compact packet brief.
  - Coder and validator turns repeatedly absorbed long `pre-work`, packet, and proof text blocks rather than short structured deltas.
- The lane still allows helper/documentation drift and then falls back to rediscovery.
  - Integration Validator correctly attempted `just integration-validator-context-brief WP-1-Workflow-Projection-Correlation-v1`.
  - That documented helper was missing from the `justfile` at the time, so the lane fell back to rereading protocol/command surfaces.
  - This is exactly the kind of command-surface ambiguity that a mechanical workflow should eliminate, not absorb.
- Dirty worktree and shared-junction noise is still too visible to the model.
  - The warnings themselves were valid.
  - The problem is that the model was forced to ingest the full long-form warning body rather than a compact count plus a short sample list.
- Closeout was still not terminally atomic.
  - Proof existed before packet/runtime/task-board/gate truth converged.
  - That created extra orchestration and reread cost, and left too much room for manual recovery instead of one deterministic closeout path.

### 16.4 Judgment

- This run proves that the repo-governance architecture is improving in structure, but it is still not mechanically efficient enough for top-tier cloud-model spend.
- The primary problem is no longer "lack of rules".
- The primary problem is "rules and evidence are still represented as giant text surfaces and recovery procedures instead of small deterministic commands, compact briefs, and atomic state transitions."
- In other words: governance strictness is not the main slowdown; governance representation and execution are.

### 16.5 Required Follow-On Governance Work

- `RGF-17`: Integration-Validator Merge Execution and Orphaned WP Prevention
- `RGF-18`: Accurate WP Token Accounting and Drift Detection
- `RGF-19`: Compact Gate Output and Artifact-First Overflow Discipline
- `RGF-20`: Context-Brief Command Parity and No-Rediscovery Enforcement
- `RGF-21`: Dirty Worktree and Shared-Junction Noise Compression
- `RGF-22`: Turn and Token Budget Enforcement for Orchestrator-Managed ACP Lanes

### 16.6 Exit Condition For The Next Trial

- Before the next orchestrator-managed smoketest trial is considered representative, the lane should be able to prove all of the following:
  - the WP token ledger matches raw `turn.completed` evidence closely enough to be trusted
  - final-lane helpers documented in the command surface actually exist and are wired into `just`
  - large dirty-worktree or packet/protocol outputs are summarized mechanically instead of dumped into model context
  - closeout ends in one machine-visible terminal authority outcome without manual packet/runtime/task-board reconciliation
  - per-role turn and token budgets can be measured and can fail loudly when the lane drifts into ambiguity

## 17. Appendage: Operator Assessment and Additional Root-Cause Findings

### 17.1 Operator Assessment

- The Operator assessment is that orchestrator-managed ACP remains materially slower than the former manual three-terminal workflow and is still burning too much token budget and real paid-model spend.
- The Operator assessment is also that governance strictness itself is not the only issue. The bigger failure is that the lane is still not mechanical enough:
  - too much waiting
  - too much repeated steering
  - too much manual or semi-manual state interpretation
  - too much latency between receipt generation and the next meaningful model action
- The Operator expectation remains:
  - strict Codex + role-protocol adherence is mandatory
  - startup authority must be explicit
  - the orchestrator-managed lane should behave more like deterministic relay and less like an intelligent supervisor continuously re-evaluating what to do next

### 17.2 Additional Root-Cause Findings

- The older manual system had one major hidden optimization: the Operator acted as a human context compressor.
  - only the relevant delta was relayed
  - models started working immediately on the fresh input
  - there was almost no broker or projection overhead
- The current ACP lane adds real control-plane latency.
  - request/result ledgers
  - session registry projection
  - runtime projection
  - notifications
  - heartbeats
  - governed `START_SESSION` / `SEND_PROMPT` steering
- The current lane still leaks too much workflow truth into text-first model context.
  - startup prompts
  - packet truth
  - runtime truth
  - task-board truth
  - notifications
  - validator evidence
  - gate output
- The orchestrator still has to reason too much.
  - infer who should act next
  - decide whether to launch, steer, or wait
  - interpret whether drift is semantic or merely projection lag
- Heartbeat and lifecycle fields still create too much semantic noise even though repo law already says heartbeat is liveness-only.
- The live ACP startup builder assigns `AGENTS.md`, role protocol, startup output, packet, `gpt-5.4`, and `model_reasoning_effort=xhigh`, but it still does not explicitly name `Handshake_Codex_v1.4.md` in the role-session startup authority string.
  - that is softer than the Operator’s manual launch expectation
- The old startup cheat sheet was also stale on topology, which likely increased mental drift.
  - it still described WP validator in the coder worktree before correction

### 17.3 Updated Judgment

- The main remaining slowdown is not "too many rules".
- The main remaining slowdown is "rules and evidence are still represented too often as large text surfaces and semi-manual orchestration instead of compact deterministic control steps."
- If governance remains equally strict but the relay becomes more event-driven and more compact, the lane should become materially cheaper and faster without losing rigor.

### 17.4 Additional Required Follow-On Governance Work

- `RGF-23`: Codex-Explicit ACP Startup Authority
- `RGF-24`: Receipt-Driven Next-Actor Relay and Steer Helper
- `RGF-25`: Heartbeat Liveness-Only Enforcement and Semantic Drift Rejection
- `RGF-26`: Compact Authority Digest and Canonical Active-Lane Brief
- `RGF-27`: Receipt/Notification Threshold Auto-Escalation for Stalled Relay

### 17.5 Additional Operator ROI Concerns After RGF-23 Through RGF-27

- Terminal WPs still produce too much historical noise in compact views.
  - closed packets can still surface old pending notifications and old token-budget drift as if they are active blockers
  - that is historically correct but operationally bad; compact views should default to live signal, not stale residue
- Token diagnostics are accurate now, but older WPs still look permanently broken because their tracked ledgers were historically incomplete.
  - this is useful for forensics, but poor for routine monitoring
  - a governed settlement/backfill path is now the highest ROI follow-up for diagnostics quality
- Relay is still semi-manual.
  - `just orchestrator-steer-next` exists, but the lane still requires the orchestrator to invoke it
  - the highest-value future speed win is event-driven happy-path relay from receipts/notifications instead of supervised polling
- Coder-validator exchange is still more prose-heavy than it should be.
  - if microtask exchange becomes more structured, validator steering can stay strong while costing fewer tokens

### 17.6 Additional Required Follow-On Governance Work

- `RGF-28`: Terminal-WP Noise Suppression and Historical Surface Gating
- `RGF-29`: Historical Token Ledger Settlement and Backfill
- `RGF-30`: Event-Driven Happy-Path Relay Automation
- `RGF-31`: Structured Coder-Validator Microtask Exchange Contract
