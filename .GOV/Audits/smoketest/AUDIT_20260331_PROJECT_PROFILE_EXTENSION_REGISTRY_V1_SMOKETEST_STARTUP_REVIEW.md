# Audit: Project Profile Extension Registry v1 Smoketest Startup Review

## METADATA
- AUDIT_ID: AUDIT-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1-SMOKETEST-STARTUP-REVIEW
- SMOKETEST_REVIEW_ID: SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1
- REVIEW_KIND: PROOF_RUN
- DATE_UTC: 2026-03-31
- AUTHOR: Codex acting as Orchestrator
- HISTORICAL_BASELINE_PACKET: NONE
- ACTIVE_RECOVERY_PACKET: WP-1-Project-Profile-Extension-Registry-v1
- LINEAGE_STATUS: NONE
- RELATED_PREVIOUS_REVIEWS:
  - AUDIT-20260329-WORKFLOW-PROJECTION-CORRELATION-V1-SMOKETEST-PROOF-RUN-REVIEW
- SCOPE:
  - first formal startup smoketest review for `WP-1-Project-Profile-Extension-Registry-v1`
  - orchestrator-managed ACP packet activation, communications bootstrap, and role-session launch behavior
  - packet truth, PREPARE truth, Task Board truth, and session-launch readiness on 2026-03-31
- RESULT:
  - PRODUCT_REMEDIATION: FAIL
  - MASTER_SPEC_AUDIT: PARTIAL
  - WORKFLOW_DISCIPLINE: FAIL
  - ACP_RUNTIME_DISCIPLINE: FAIL
  - MERGE_PROGRESSION: FAIL
- KEY_COMMITS_REVIEWED:
  - NONE
- EVIDENCE_SOURCES:
  - `.GOV/Audits/smoketest/AUDIT_20260329_WORKFLOW_PROJECTION_CORRELATION_V1_SMOKETEST_PROOF_RUN_REVIEW.md`
  - `.GOV/refinements/WP-1-Project-Profile-Extension-Registry-v1.md`
  - `.GOV/task_packets/WP-1-Project-Profile-Extension-Registry-v1/packet.md`
  - `../gov_runtime/roles_shared/ORCHESTRATOR_GATES.json`
  - `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Project-Profile-Extension-Registry-v1/RUNTIME_STATUS.json`
  - `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Project-Profile-Extension-Registry-v1/RECEIPTS.jsonl`
  - `../gov_runtime/roles_shared/GATE_OUTPUTS/pre-work/WP-1-Project-Profile-Extension-Registry-v1/2026-03-31T17-21-13-078Z.log`
  - `../gov_runtime/roles_shared/GATE_OUTPUTS/pre-work/WP-1-Project-Profile-Extension-Registry-v1/2026-03-31T17-21-37-098Z.log`
  - `.GOV/roles_shared/scripts/wp/ensure-wp-communications.mjs`
  - live orchestrator command sequence on 2026-03-31
- RELATED_GOVERNANCE_ITEMS:
  - RGF-38
  - RGF-39
  - RGF-40
- RELATED_CHANGESETS:
  - GOV-CHANGE-20260331-01
  - GOV-CHANGE-20260331-02

---

## 1. Executive Summary

- This WP did not start cleanly.
- The refinement, signature, PREPARE, packet creation, Task Board activation, and pre-work gate were eventually brought into a valid state.
- That is not enough. The orchestrator stopped after reaching `READY_FOR_DEV` and did not autonomously launch the governed ACP role sessions, which is a workflow failure for an `ORCHESTRATOR_MANAGED` lane.
- A separate governance/runtime defect also blocked first activation: WP communication bootstrap wrote invalid JSON because runtime-status placeholders were left unreplaced.
- Net judgment: packet activation recovered, but workflow discipline failed and the product WP had not actually started at the point the operator had to intervene.

## 2. Lineage and What This Run Needed To Prove

- This is the first formal smoketest review for `WP-1-Project-Profile-Extension-Registry-v1`.
- Relative to the 2026-03-29 workflow-projection proof-run review, this run needed to prove that the refreshed orchestrator-managed ACP lane could do the next high-priority product WP without falling back into operator babysitting.
- The exact proof target was:
  - activate the WP cleanly
  - create valid communications artifacts
  - run pre-work in the assigned coder worktree
  - start the needed governed role sessions through terminal/ACP using the correct role startup surfaces
  - avoid stopping at a packet-only state that still requires operator correction

### What Improved vs Previous Smoketest

- The targeted product gap is now packeted and activation-ready instead of remaining only a stub.
- The workflow surfaced a real communications-bootstrap bug immediately instead of silently proceeding with broken runtime artifacts.
- The refinement and signature ordering stayed governed.
- What did not improve enough:
  - the orchestrator still did not autonomously complete the start-of-work sequence
  - the operator still had to restate a core lane expectation
  - command-surface truth still drifted enough that the suggested helper `just ensure-wp-communications` did not actually exist

## 3. Product Outcome

- No product remediation work had started at the failure point.
- The product scope is correctly packeted, signed, prepared, and pre-work-clean.
- The signed packet scope remains the right remediation target:
  - explicit project-profile extension registry closure
  - Task Board propagation of the base-envelope versus `profile_extension` boundary
  - Role Mailbox propagation of the same boundary
  - software and non-software emitted-artifact proof plus unknown-extension fallback
- Product judgment for this review remains `FAIL` because startup proof is about actual governed execution, not only paperwork readiness.

## 4. Timeline

- Refinement was created, repaired to satisfy the modern refinement gate, shown in chat, approved, and signed.
- `just orchestrator-prepare-and-packet WP-1-Project-Profile-Extension-Registry-v1` initially failed because WP communications bootstrap produced invalid JSON.
- The root cause was repaired in `.GOV/roles_shared/scripts/wp/ensure-wp-communications.mjs`.
- Packet creation then landed:
  - PREPARE recorded
  - packet directory created
  - Task Board moved to `READY_FOR_DEV`
  - communications artifacts created
- `pre-work` failed once because it was run from `wt-gov-kernel` instead of the assigned coder worktree.
- `pre-work` then passed from `wtc-extension-registry-v1`.
- The orchestrator still did not launch the coder and WP validator sessions before the operator had to call out the failure.

## 5. Failure Inventory

### 5.1 Critical: the orchestrator did not actually start the WP after activation

Evidence:

- `just orchestrator-next WP-1-Project-Profile-Extension-Registry-v1` reported `STAGE: DELEGATION` and printed the correct next commands:
  - `just launch-coder-session WP-1-Project-Profile-Extension-Registry-v1`
  - `just launch-wp-validator-session WP-1-Project-Profile-Extension-Registry-v1`
- those sessions were not launched before the operator had to intervene

Reason:

- the orchestrator stopped at packet readiness instead of completing the governed startup sequence for an `ORCHESTRATOR_MANAGED` lane

Impact:

- the WP was described as started when it was only packet-ready
- the operator had to supply a corrective instruction for a step that should have been autonomous

Judgment:

- this is the primary workflow failure in the run

### 5.2 High: first activation failed because WP communications bootstrap emitted invalid JSON

Evidence:

- `just orchestrator-prepare-and-packet WP-1-Project-Profile-Extension-Registry-v1` blocked with:
  - `Expected property name or '}' in JSON at position 1379 (line 24 column 41)`
- `.GOV/templates/WP_RUNTIME_STATUS_TEMPLATE.json` contained placeholders for:
  - `CURRENT_MAIN_COMPATIBILITY_STATUS`
  - `CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA`
  - `CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC`
  - `PACKET_WIDENING_DECISION`
  - `PACKET_WIDENING_EVIDENCE`
- `.GOV/roles_shared/scripts/wp/ensure-wp-communications.mjs` did not fill those placeholders before validation

Reason:

- the communications bootstrap template and replacement map had drifted apart

Impact:

- first packet activation aborted
- orchestrator had to repair governance tooling mid-run before the WP could even reach pre-work

Judgment:

- this is a real ACP/runtime control-plane defect, not an incidental typo

### 5.3 Medium: pre-work was initially run from the wrong worktree

Evidence:

- `../gov_runtime/roles_shared/GATE_OUTPUTS/pre-work/WP-1-Project-Profile-Extension-Registry-v1/2026-03-31T17-21-13-078Z.log`
- failure lines:
  - expected branch `feat/WP-1-Project-Profile-Extension-Registry-v1`, got `gov_kernel`
  - expected worktree `../wtc-extension-registry-v1`, current `wt-gov-kernel`

Reason:

- the orchestrator used a valid command on the wrong governed surface

Impact:

- another avoidable repair loop happened before the actual coder lane was started

Judgment:

- this is not as severe as the missing launch, but it is still workflow misuse that increased operator/token cost

### 5.4 Medium: the prescribed repair command did not exist on the just command surface

Evidence:

- blocked packet creation printed:
  - `just ensure-wp-communications WP-1-Project-Profile-Extension-Registry-v1`
- running that command failed because the justfile had no such recipe

Reason:

- command-surface guidance drifted from actual justfile availability

Impact:

- repair required direct script execution instead of the promised governed helper

Judgment:

- this is command-surface drift and should be treated as governance debt

## 6. Role Review

### 6.1 Orchestrator Review

Strengths:

- refinement/signature ordering stayed governed
- the communications-bootstrap root cause was diagnosed and repaired instead of being papered over
- packet creation, PREPARE truth, communications truth, and pre-work truth were eventually aligned

Failures:

- did not autonomously launch the governed ACP coder and WP-validator sessions
- ran `pre-work` from the wrong worktree first
- relied on a printed repair command that was not actually present on the just surface

Assessment:

- FAIL. The orchestrator got the WP to a valid ready state, but did not complete the start-of-work responsibility for an orchestrator-managed lane.

### 6.2 Coder Review

Strengths:

- NONE; coder session was not started at the failure point being reviewed

Failures:

- NONE attributable to the coder; the lane was not launched yet

Assessment:

- NOT RUN

### 6.3 WP Validator Review

Strengths:

- NONE; validator session was not started at the failure point being reviewed

Failures:

- NONE attributable to the validator; the lane was not launched yet

Assessment:

- NOT RUN

### 6.4 Integration Validator Review

Strengths:

- NONE; integration validation is downstream and not applicable at startup

Failures:

- NONE

Assessment:

- NOT APPLICABLE

## 7. Review Of Coder and Validator Communication

- No direct review traffic existed yet because the governed role sessions had not been launched.
- That absence is itself a workflow failure for the startup phase.

## 8. ACP Runtime / Session Control Findings

- The ACP/runtime surface failed first at communications bootstrap because runtime-status JSON could not be generated deterministically.
- After repair, communications artifacts were valid and the packet-declared communication directory became live.
- ACP launch itself still had not happened at the point the operator intervened, so runtime/session-control discipline remains a FAIL for this review.

## 9. Governance Implications

- `ORCHESTRATOR_MANAGED` cannot be treated as satisfied by packet activation alone; session launch is part of actual start-of-work.
- The command surface must not print repair helpers that do not exist.
- Runtime bootstrap templates and replacement maps need a regression guard so packet activation cannot silently drift back into malformed JSON.

## 10. Positive Signals Worth Preserving

- The refinement gate forced a real spec-grounded remediation brief before activation.
- Signature bundle, PREPARE tuple, packet creation, Task Board status, and pre-work all converged onto the intended governed shape.
- The failed bootstrap was traceable enough that the concrete bug could be repaired quickly.

## 11. Remaining Product or Spec Debt

- The actual product remediation work for `WP-1-Project-Profile-Extension-Registry-v1` has not yet begun in code.
- The adjacent product debt from the prior review remains:
  - no explicit project-profile extension registry proof in product code
  - Task Board still flattens to `software_delivery`
  - Role Mailbox still flattens to `software_delivery`
  - no non-software emitted-artifact proof exists yet

## 12. Post-Smoketest Improvement Rubric

### 12.1 Workflow Smoothness

- TREND: REGRESSED
- CURRENT_STATE: HIGH
- Evidence:
  - operator had to restate that the WP should have been started autonomously
  - first activation failed on governance/runtime bootstrap
  - `pre-work` was run from the wrong worktree
  - commanded repair surface drifted from actual just availability
- What improved:
  - packet/pre-work truth eventually converged
- What still hurts:
  - the orchestrator still did not finish the governed startup sequence without operator correction
- Next structural fix:
  - make `orchestrator-managed` delegation incomplete until required role sessions have actually been launched and registered

### 12.2 Master Spec Gap Reduction

- TREND: FLAT
- CURRENT_STATE: HIGH
- Evidence:
  - packet scope is correct and explicit
  - no product implementation work ran yet
- What improved:
  - the gap is now packeted with a specific governed implementation brief
- What still hurts:
  - none of the product acceptance points are closed yet
- Next structural fix:
  - execute the governed coder + validator lanes immediately after pre-work PASS so the product remediation actually begins

### 12.3 Token Cost Pressure

- TREND: REGRESSED
- CURRENT_STATE: HIGH
- Evidence:
  - repeated protocol rereads and command-surface discovery
  - failed activation, repair loop, wrong-worktree pre-work, and explicit operator restatement
  - command-surface mismatch around `just ensure-wp-communications`
- What improved:
  - once the root cause was found, the repair was concrete and local
- What still hurts:
  - startup is still too repair-heavy for a workflow that claims governed autonomy
- Next structural fix:
  - add a regression test that packet activation plus communications bootstrap plus first pre-work all succeed end to end before an orchestrator-managed lane is treated as launch-ready

## 13. Silent Failures, Command Surface Misuse, and Ambiguity Scan

### 13.1 Silent Failures / False Greens

- The WP looked "started" because it had a packet and `READY_FOR_DEV` Task Board state, but no governed role sessions had been launched yet.
- The first `pre-work` failure showed that being packet-ready on `gov_kernel` is not the same thing as being lane-ready in the assigned coder worktree.

### 13.2 Systematic Wrong Tool or Command Calls

- The orchestrator did not issue the required `launch-coder-session` and `launch-wp-validator-session` calls after reaching `DELEGATION`.
- The suggested repair command `just ensure-wp-communications` was invalid on the current just surface.
- `pre-work` was first run from the wrong worktree.

### 13.3 Task and Path Ambiguity

- There was too much ambiguity between:
  - packet truth in `wt-gov-kernel`
  - execution truth in `wtc-extension-registry-v1`
  - external communication runtime truth in `../gov_runtime/roles_shared/WP_COMMUNICATIONS/...`

### 13.4 Read Amplification / Governance Document Churn

- Multiple rereads were required across:
  - Orchestrator protocol
  - just command surface
  - create-task-packet behavior
  - communications bootstrap scripts
- That reread load was caused by workflow ambiguity, not by novel product complexity.

### 13.5 Hardening Direction

- Add a real `just ensure-wp-communications` helper matching the printed remediation surface.
- Add an orchestrator-managed launch-completeness gate so `DELEGATION` requires actual ACP session startup, not only a printed next-command list.
- Add regression coverage for packet activation JSON template fill completeness.

## 14. Suggested Remediations

### Governance / Runtime

- Add `just ensure-wp-communications WP-{ID}` to the just command surface.
- Add a launch-completeness gate: after `pre-work` PASS on an orchestrator-managed WP, the orchestrator must start coder and WP validator sessions or the lane remains workflow-invalid.
- Add a regression test covering runtime-status template placeholder completeness.

### Product / Validation Quality

- NONE in this review beyond starting the governed sessions immediately after packet readiness; product work itself remains the next step.

### Documentation / Review Practice

- Update startup and delegation guidance to state explicitly that packet activation is not equivalent to "WP started" on an orchestrator-managed lane.
- Future startup reviews should separate:
  - packet readiness
  - worktree readiness
  - session-launch readiness
  - actual ACP session launch

## 15. Command Log

- `just record-refinement WP-1-Project-Profile-Extension-Registry-v1` -> PASS
- `just record-signature WP-1-Project-Profile-Extension-Registry-v1 ilja310320261913 ORCHESTRATOR_MANAGED Coder-A` -> PASS
- `just orchestrator-prepare-and-packet WP-1-Project-Profile-Extension-Registry-v1` -> FAIL (first attempt; invalid JSON during communications bootstrap)
- `node .GOV/roles_shared/scripts/wp/ensure-wp-communications.mjs WP-1-Project-Profile-Extension-Registry-v1` -> PASS (after repair)
- `just pre-work WP-1-Project-Profile-Extension-Registry-v1` from `wt-gov-kernel` -> FAIL (wrong worktree)
- `just pre-work WP-1-Project-Profile-Extension-Registry-v1` from `wtc-extension-registry-v1` -> PASS
- `just orchestrator-next WP-1-Project-Profile-Extension-Registry-v1` -> PASS (`DELEGATION`; coder + WP validator launch required next)

## 16. Live Continuation Addendum (2026-03-31)

### 16.1 Additional Failure Findings After the WP Actually Started

#### 16.1.1 High: launch remained split from actual governed ACP start

- The startup failure was not limited to packet activation.
- Even after the packet was valid and `pre-work` passed, the lane still did not become live until the governed `start-*` surface was issued explicitly after the `launch-*` calls.
- That means the launch surface still allowed a false-green state where a WP looked delegated while governed ACP threads were not yet registered.
- This confirms the workflow defect is structural, not only operator misuse.

#### 16.1.2 High: the first coder pass failed signed-scope and Master-Spec fidelity review

- Once the WP validator lane actually ran, the first governed product pass was rejected.
- The validator review at `2026-03-31T18:51:06.744Z` recorded four concrete failures:
  - no explicit extension-registry contract yet in `src/backend/handshake_core/src/locus/types.rs`
  - Task Board parity still incomplete because `src/backend/handshake_core/src/locus/task_board.rs` remained outside the actual product diff
  - Role Mailbox parity still incomplete because the profile-extension boundary and unknown-extension behavior were not proven
  - signed-scope drift because the diff touched `src/backend/handshake_core/src/storage/locus_sqlite.rs` outside the signed packet scope
- This is product-code failure, but it is also a governance accuracy signal because the governed lane still allowed a first pass that was not close enough to the signed packet and Master Spec contract.

#### 16.1.3 Medium: runtime projection still lagged live receipt truth after review traffic landed

- After `VALIDATOR_KICKOFF`, `CODER_INTENT`, `CODER_HANDOFF`, and `VALIDATOR_REVIEW` all existed in `RECEIPTS.jsonl`, `RUNTIME_STATUS.json` still reported:
  - `current_packet_status: Ready for Dev`
  - `runtime_status: submitted`
  - `current_phase: BOOTSTRAP`
  - `waiting_on: FINAL_REVIEW_EXCHANGE`
- That runtime state no longer described the real lane state.
- The governed workflow therefore still has a projection lag problem between receipt truth and runtime summary truth.

#### 16.1.4 Medium: stalled-relay recovery still required explicit orchestrator steering

- Before the validator review landed, relay health escalated and required explicit `just orchestrator-steer-next WP-1-Project-Profile-Extension-Registry-v1`.
- The system did detect the stall, which is better than silent waiting.
- But it still did not complete the happy-path relay autonomously enough to avoid extra orchestrator intervention.

### 16.2 Governance Implication of the Continuation Run

- The continuation run proves the original startup review was directionally correct but incomplete.
- The repo-governance failure is now broader than "packet bootstrap broke once."
- The actual failure set is:
  - command-surface parity drift around communications repair
  - launch-to-start autonomy gap on orchestrator-managed ACP lanes
  - lagging packet/runtime projection after direct-review receipts
  - insufficient first-pass accuracy against the signed product packet

### 16.3 Additional Remediation Items Opened From This Addendum

- `RGF-38` - Autonomous Launch-to-Start Convergence
- `RGF-39` - WP Communications Helper Parity and Template Completeness
- `RGF-40` - Runtime / Relay Projection Convergence After Review Traffic
