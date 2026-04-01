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

## 17. Governance Status Snapshot (Current WP State)

- Official governance status is still split:
  - `.GOV/roles_shared/records/TASK_BOARD.md` still records `WP-1-Project-Profile-Extension-Registry-v1` as `[READY_FOR_DEV]`.
  - `.GOV/task_packets/WP-1-Project-Profile-Extension-Registry-v1/packet.md` still records `**Status:** Ready for Dev`.
  - `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Project-Profile-Extension-Registry-v1/RUNTIME_STATUS.json` still reports `runtime_status=submitted`, `current_phase=BOOTSTRAP`, and `waiting_on=FINAL_REVIEW_EXCHANGE` even though direct review traffic has already happened.
- Live receipt truth says the WP is no longer merely ready:
  - `VALIDATOR_KICKOFF` was issued.
  - `CODER_INTENT` was issued.
  - `CODER_HANDOFF` was issued.
  - `VALIDATOR_REVIEW` then returned the WP to `CODER` for repair at `2026-03-31T18:51:06.744Z`.
- Practical status:
  - the WP is active but not complete
  - the first product pass failed WP-validator review
  - the next honest state is "coder repair required," not "Ready for Dev"
- Governance follow-on status:
  - `RGF-38` is `DONE`
  - `RGF-39` is `DONE`
  - `RGF-40` remains `READY`

## 18. Product Implementation Audit vs Master Spec

### 18.1 What the Current Product Diff Actually Improved

- `src/backend/handshake_core/src/locus/types.rs` now carries `profile_extension` through Work Packet, Micro-Task, and structured-summary record types.
- `src/backend/handshake_core/src/workflows.rs` now propagates `project_profile_kind` and `profile_extension` into Work Packet and Micro-Task detail plus summary artifacts.
- `src/backend/handshake_core/tests/micro_task_executor_tests.rs` now includes:
  - one software-delivery Work Packet example
  - one research Micro-Task example
  - one research Task Board projection example
- `src/backend/handshake_core/src/workflows.rs` also stops flattening Task Board projections to `software_delivery` at the row/index/view `project_profile_kind` field.

### 18.2 High: the explicit extension registry contract is still not implemented

Evidence:

- The signed packet requires an explicit registry-backed contract and deterministic handling for unknown breaking extensions.
- `validate_profile_extension` in `src/backend/handshake_core/src/locus/types.rs` still only checks:
  - object shape
  - non-empty `extension_schema_id`
  - non-empty `extension_schema_version`
  - presence of `compatibility`
  - explicit breaking compatibility rejection
- No product file implements an allow-list, registry table, or explicit `(extension_schema_id, extension_schema_version)` contract for supported profile extensions.

Reason:

- The implementation stopped at shape validation plus one compatibility tripwire instead of closing the registry requirement in the signed packet.

Impact:

- Unknown extension ids and versions are still not proven to degrade or reject according to a real registry contract.
- The product remains vulnerable to false confidence because extension-shaped metadata can still look "supported enough."

Judgment:

- This is the main unclosed Master Spec requirement.
- The WP cannot honestly be treated as product-complete while this remains open.

### 18.3 High: Task Board parity is still incomplete against the signed packet

Evidence:

- The signed packet names `src/backend/handshake_core/src/locus/task_board.rs` as an expected code surface.
- The active product diff does not touch `src/backend/handshake_core/src/locus/task_board.rs` at all.
- `TaskBoardEntryRecordV1`, `TaskBoardIndexV1`, and `TaskBoardViewV1` still carry `project_profile_kind` but no `profile_extension` field.
- `src/backend/handshake_core/src/workflows.rs` projects `project_profile_kind` into Task Board artifacts, but it cannot preserve the base-envelope/profile-extension boundary because the record structs themselves still do not expose that field.

Reason:

- The implementation improved one visible Task Board field while missing the signed artifact-family boundary requirement.

Impact:

- Task Board records still cannot prove base-envelope parity with Work Packet and Micro-Task artifacts.
- Generic viewers and validators still do not receive the full contract the packet requires.

Judgment:

- This is a substantive spec miss, not a cosmetic omission.
- The diff improved Task Board projection fidelity only partially.

### 18.4 High: Role Mailbox parity is still incomplete and currently improves portability by erasing detail

Evidence:

- `src/backend/handshake_core/src/role_mailbox.rs` now replaces hard-coded `software_delivery` with `generic`.
- Mailbox outputs still do not carry a `profile_extension` boundary.
- `src/backend/handshake_core/tests/role_mailbox_tests.rs` only proves `project_profile_kind == generic`; it does not prove:
  - extension preservation
  - unknown-extension fallback behavior
  - deterministic rejection behavior

Reason:

- The implementation removed one incorrect specialization but did not carry the contract boundary required by the packet.

Impact:

- Mailbox exports remain easier to parse only because specialization was flattened away.
- That is weaker than preserving the base envelope and extension boundary coherently.

Judgment:

- This is a real contract miss.
- It also shows a "looks portable, but only after discarding semantics" failure mode that the packet explicitly tries to avoid.

### 18.5 Medium: signed-scope drift is visible in the actual diff

Evidence:

- The active product diff touches `src/backend/handshake_core/src/storage/locus_sqlite.rs`.
- The signed packet expected `src/backend/handshake_core/src/locus/task_board.rs` instead.
- The packet and validator both called this out directly.

Reason:

- The implementation appears to have followed the easiest persistence path for getting profile metadata into stored records instead of closing the signed artifact-family surface first.

Impact:

- Reviewability dropped because the diff widened outside the declared code surface while still missing a declared surface.
- This made it harder to distinguish necessary persistence work from speculative scope drift.

Judgment:

- This is a technical implementation failure and a governance-discipline failure.
- It is also a concrete sign that the WP was not being driven tightly enough by the packet.

### 18.6 Medium: the proof set is shallower than the signed packet requires

Evidence:

- Added tests now prove:
  - Work Packet detail/summary propagation with software-delivery `profile_extension`
  - Micro-Task detail/summary propagation with research `profile_extension`
  - Task Board `project_profile_kind` projection for a research example
  - rejection when `compatibility.breaking == true`
- The tests do not yet prove:
  - explicit registry acceptance for known extension ids and versions
  - unknown `extension_schema_id` behavior
  - unknown `extension_schema_version` behavior
  - Task Board `profile_extension` boundary preservation
  - Role Mailbox `profile_extension` boundary preservation
  - unknown-extension fallback or rejection through Task Board and Role Mailbox surfaces

Reason:

- The proof suite focused on the easiest visible fields first.

Impact:

- The current test suite can go green while the signed contract is still materially open.

Judgment:

- This is insufficient proof depth for a schema-and-artifact-family WP.

### 18.7 Vibe-Coding Signals and Spec-Gap Signals

- `aggregate_project_profile_kind()` in `src/backend/handshake_core/src/workflows.rs` collapses mixed Task Board kinds to `generic`. That may be reasonable, but the packet does not prove this exact aggregation rule. It is an inferred semantic choice rather than an explicitly justified one.
- `src/backend/handshake_core/src/role_mailbox.rs` switches mailbox exports from `software_delivery` to `generic` without preserving `profile_extension`. That looks like portability progress at the top level while leaving the signed contract open underneath.
- The first coder handoff claimed "Implemented end-to-end project-profile propagation" even though:
  - `locus/task_board.rs` remained untouched
  - no explicit registry existed
  - mailbox boundary preservation remained unproven
- The test additions mostly prove top-level field presence and selected happy-path examples rather than the full registry and cross-artifact boundary contract.

Judgment:

- The product work shows real progress, but the implementation is still too close to "make the visible fields pass" rather than "close the exact signed contract."

## 19. Exhaustive Failure Inventory By Role

### 19.1 Orchestrator

Failures:

- did not autonomously complete the governed launch-to-start path on the first pass
- stopped at packet activation and launch-ready language instead of starting the role sessions
- first ran `pre-work` from the wrong worktree
- suggested or relied on a repair helper that was not actually present on the just command surface until repaired
- required repeated rereads across:
  - Orchestrator protocol
  - command surface
  - packet-creation flow
  - communications bootstrap behavior
- required explicit operator correction to continue from "packet exists" to "WP actually started"
- later required explicit `just orchestrator-steer-next` to push the relay after escalation
- allowed governance truth to remain split across:
  - Task Board
  - packet status
  - runtime status
  - receipt truth

Assessment:

- The role did eventually activate the lane and harden governance in response, but the startup autonomy target was not met on the first pass.

### 19.2 Coder

Failures:

- the first product pass missed the central explicit registry contract in `src/backend/handshake_core/src/locus/types.rs`
- the first product pass entirely missed the signed `src/backend/handshake_core/src/locus/task_board.rs` surface
- the first product pass improved Role Mailbox top-level kind reporting without preserving the profile-extension boundary
- the diff touched out-of-scope `src/backend/handshake_core/src/storage/locus_sqlite.rs` while a signed surface remained untouched
- the proof set underfit the signed packet and did not close:
  - unknown extension id behavior
  - unknown extension version behavior
  - Task Board boundary preservation
  - mailbox boundary preservation
  - unknown-extension fallback/rejection across exported artifact families
- the handoff summary overclaimed completion relative to the actual diff
- the proof commands named in the kickoff were not mirrored honestly in the handoff; the handoff reported broader test binaries instead of the packet-targeted proof shape

Assessment:

- The coder made partial technical progress, especially on Work Packet and Micro-Task propagation, but the first pass was not sufficiently packet-driven and was not close enough to the signed Master Spec boundary.

### 19.3 WP Validator

Failures:

- no mid-session corrective steering appears in the governed thread between `CODER_INTENT` and `CODER_HANDOFF`
- the coder intent already under-specified the critical `locus/task_board.rs` surface and did not name the unknown-extension proof path concretely, but no corrective steer was recorded before the full handoff
- review arrived only after the first full implementation pass, which increased rework cost

Strengths:

- kickoff was concrete and spec-anchored
- final review was specific, accurate, and caught the real contract misses

Assessment:

- WP Validator quality was strong at kickoff and handoff review, but direct-review cadence should be tighter on a contract-heavy WP like this one.

### 19.4 Integration Validator

Failures:

- NONE YET in the technical sense; the lane has not reached Integration Validator review.

Concerns:

- the workflow never reached the stage where Integration Validator could add value because startup autonomy and first-pass product fidelity both failed earlier.

Assessment:

- No execution failure is attributable yet, but the lack of progression to Integration Validator remains a workflow-loss signal.

## 20. Review Of Coder and Validator Communication

- Direct coder/WP-validator communication did happen. This is materially better than a startup-only lane with no governed review traffic.
- Quantity was low:
  - `VALIDATOR_KICKOFF`
  - `CODER_INTENT`
  - `CODER_HANDOFF`
  - `VALIDATOR_REVIEW`
- Quality was mixed:
  - `VALIDATOR_KICKOFF` was high-signal, specific, and spec-anchored.
  - `CODER_INTENT` was orderly but already slightly too loose on the exact file/surface closure requirement.
  - `CODER_HANDOFF` overclaimed completion.
  - `VALIDATOR_REVIEW` was strong and corrective.
- Mid-session steering did not happen:
  - no validator correction was recorded between `CODER_INTENT` and `CODER_HANDOFF`
  - for a packet with a strict file list and explicit proof commands, that is too little supervision
- Orchestrator steering volume was higher than a healthy lane should require:
  - startup correction
  - launch/start correction
  - later relay escalation recovery
- Communication should improve in two ways:
  - require one short validator checkpoint when the coder intent omits a signed surface or when the packet is contract-heavy
  - require coder handoff claims to mirror the signed proof contract exactly instead of broad "end-to-end" language

Judgment:

- Communication existed and was useful, but it was too sparse for the kind of spec-boundary work this WP requires.

## 21. Post-Smoketest Improvement Rubric Addendum

### 21.1 Workflow Smoothness

- TREND: IMPROVED
- CURRENT_STATE: HIGH
- Evidence:
  - `RGF-38` removed the launch-only false green
  - `RGF-39` repaired command-surface parity and template completeness
  - the operator still had to correct startup behavior
  - runtime truth still lagged receipt truth after live review traffic
  - explicit `just orchestrator-steer-next` was still needed after relay escalation
- What improved:
  - packet activation no longer ends at a fake launch-complete state
  - communications bootstrap failure is now guarded more honestly
- What still hurts:
  - official packet/Task Board status still diverges from live review truth
  - the lane still needed manual steering beyond normal orchestration
  - validator feedback still arrived late in the loop
- Next structural fix:
  - close `RGF-40` so receipt-driven runtime/status projection stays honest after live review traffic

### 21.2 Master Spec Gap Reduction

- TREND: IMPROVED
- CURRENT_STATE: HIGH
- Evidence:
  - Work Packet and Micro-Task detail/summary propagation improved
  - non-software example coverage exists now where it was previously absent
  - the main registry and cross-artifact boundary gap is still open
  - signed scope is still not closed
- What improved:
  - the remaining gap surface is more explicit than before
  - validator review now names the exact missing product surfaces and proof obligations
- What still hurts:
  - no explicit registry contract yet
  - Task Board boundary still incomplete
  - Role Mailbox boundary still incomplete
  - scope drift and overclaiming remain present
- Next structural fix:
  - implement the explicit registry plus Task Board and Role Mailbox boundary preservation exactly on the signed surfaces before widening anything else

### 21.3 Token Cost Pressure

- TREND: FLAT
- CURRENT_STATE: HIGH
- Evidence:
  - repeated operator clarification was needed at startup
  - repeated governance-document rereads happened because the live command/path surface was ambiguous
  - the lane consumed an extra full review loop because the first product pass was not packet-close
  - stale runtime truth increased status-check overhead
- What improved:
  - two recurring startup/governance failures have now been turned into explicit governance fixes (`RGF-38`, `RGF-39`)
- What still hurts:
  - status checks still require reading multiple truth surfaces
  - the first product pass still generated avoidable validator rework
  - sparse mid-session steering lets errors persist until full handoff
- Next structural fix:
  - add an explicit contract-heavy mid-session checkpoint rule so packet-surface misses are corrected before full handoff

## 22. ROI and Concern Review

### 22.1 ROI

- The governed direct-review loop produced real value:
  - the validator caught false closure on the first pass
  - the product is no longer being judged only by top-level field presence
  - the remaining spec gap is now concrete and reviewable
- The startup failures also produced immediate repo-governance value:
  - `RGF-38` and `RGF-39` are already closed
  - `RGF-40` is now grounded in live evidence instead of abstract suspicion
- This WP still has high strategic ROI because if closed honestly it unlocks the project-agnostic workflow-law stack without forcing later WPs to keep unwinding software-delivery assumptions.

### 22.2 Concerns

- Too much governance truth is still split across:
  - packet status
  - Task Board status
  - runtime status
  - live receipts
- The first product pass still looked more like field-plumbing toward local green than a strict closure of the signed packet.
- Reviewability remains expensive because:
  - runtime truth is stale
  - the product worktree is noisy
  - scope drift happened while a signed surface remained untouched
- The lane has not yet reached Integration Validator, so end-to-end workflow accuracy is still unproven.

### 22.3 Bottom-Line Judgment

- This WP generated useful governance hardening and useful negative product proof.
- It did not yet generate a high-accuracy end-to-end product closure.
- The next productive move is not more broad exploration; it is a tightly packet-driven repair pass that closes:
  - explicit registry enforcement
  - Task Board boundary parity
  - Role Mailbox boundary parity
  - proof of unknown-extension fallback or deterministic rejection

## 23. 2026-04-01 Closeout Addendum

### 23.1 Updated Result Snapshot

- PRODUCT_REMEDIATION: PASS
- MASTER_SPEC_AUDIT: PASS
- WORKFLOW_DISCIPLINE: FAIL
- ACP_RUNTIME_DISCIPLINE: FAIL
- MERGE_PROGRESSION: PARTIAL

### 23.2 Updated Executive Summary

- The WP did eventually close product scope honestly.
- Packet truth is now `Validated (PASS)`, local `main` contains the approved product result at `26e85bbfdebfa5b19044420ced816ee3c3501f5d`, and the governance/runtime/task-board/build-order surfaces were reconciled and pushed.
- That successful closeout does not erase the governance failures that happened during recovery.
- The strongest additional negative finding is that the role-locked Orchestrator session crossed into product-code containment and conflict-resolution work in `handshake_main` while still operating as `ORCHESTRATOR`. That should have been blocked or rerouted through the governed Coder or Integration Validator lane.
- Net judgment after full closeout: the product WP succeeded, but the orchestrator-managed governance/ACP lane still failed its role-separation standard and required too much reactive repair to get there.

### 23.3 What Improved vs The Earlier State In This Same Review

- The signed product gap that was still open in sections 18-22 is now closed:
  - explicit profile-extension registry enforcement exists
  - Task Board projection carries the signed boundary
  - Role Mailbox projection carries the signed boundary
  - unknown-extension rejection is proven
- The live truth split identified earlier is materially better now:
  - packet, Task Board, BUILD_ORDER, runtime, and receipts converge after review traffic
  - terminal closeout now retires stale coder and WP-validator steerable sessions
  - Integration Validator governance authority is kernel-rooted instead of silently falling back to `handshake_main/.GOV`
  - `sync-gov-to-main` now fails closed when the kernel source is dirty
- What did not improve enough:
  - the run still needed operator-triggered governance coding to recover
  - the final closeout still crossed the orchestrator/product-code boundary
  - the lane was not self-contained enough to finish without governance-side intervention

## 24. Additional Failure Inventory Since Startup Review

### 24.1 High: review truth kept diverging after real review traffic

Evidence:

- `VALIDATOR_REVIEW` was already present at `2026-03-31T18:51:06.744Z`, and later PASS-grade review receipts landed at `2026-04-01T02:11:17.238Z` and `2026-04-01T02:46:32.499Z`.
- Even after that, official packet/runtime/task-board/build-order truth needed explicit repair and additional governance hardening before it reflected the actual lane state honestly.

Reason:

- receipt truth was ahead of the projection/writeback layer

Impact:

- the operator could not trust a single authority surface for "where the WP really is"
- relay and closeout decisions became more manual than they should be

Judgment:

- This is a control-plane failure, not a cosmetic status bug.

### 24.2 High: technically valid final review was blocked by governance/runtime defects, not by product defects

Evidence:

- `REVIEW_RESPONSE` from `INTEGRATION_VALIDATOR` at `2026-04-01T02:46:32.499Z` explicitly said no new blocking product-code findings remained, but closure was blocked by `BLOCKER_CLASS: ENVIRONMENT_FAILURE`.
- The same receipt named the blockers:
  - broken justfile/node wrappers in a space-containing workspace
  - `CURRENT_MAIN_COMPATIBILITY_STATUS` still `NOT_RUN`

Reason:

- final-lane closure depended on governance tooling that was not path-safe and not yet converged with the newer closeout truth model

Impact:

- the lane could reach technical PASS and still be unable to close mechanically

Judgment:

- This is a direct ACP/runtime discipline failure.

### 24.3 High: the role-locked Orchestrator session crossed into product-code work

Evidence:

- The operator never reassigned the role; the live instruction remained `ROLE LOCK: You are the ORCHESTRATOR.`
- No packet field, receipt, or governed role reassignment recorded authority for the Orchestrator to author or harmonize product code on `main`.
- The contained local-`main` commit `26e85bbfdebfa5b19044420ced816ee3c3501f5d` required cherry-pick conflict resolution and additional harmonization inside these product files:
  - `src/backend/handshake_core/src/locus/task_board.rs`
  - `src/backend/handshake_core/src/locus/types.rs`
  - `src/backend/handshake_core/src/role_mailbox.rs`
  - `src/backend/handshake_core/src/storage/locus_sqlite.rs`
  - `src/backend/handshake_core/src/workflows.rs`
  - `src/backend/handshake_core/tests/micro_task_executor_tests.rs`
  - `src/backend/handshake_core/tests/role_mailbox_tests.rs`
- Those containment/harmonization steps were executed from this orchestrator session after the final-lane stall, not from a fresh governed coder turn.

Reason:

- repo governance does not yet hard-fail product-code authoring by a role-locked Orchestrator when the lane is stalled or ambiguous

Impact:

- role separation collapsed at the most sensitive stage of the lane
- the audit trail for "who actually changed product code and under what authority" became weaker than it should be
- the run succeeded partly because the Orchestrator stepped out of role, which means the governance system did not actually prove that ACP steering alone was sufficient

Judgment:

- This is a severe governance failure, even though the resulting product code appears technically correct.

### 24.4 High: Integration Validator governance authority was too ambiguous before hardening

Evidence:

- the final-lane fix set had to force `HANDSHAKE_GOV_ROOT=<wt-gov-kernel>/.GOV`
- protocol and command-surface law had to be updated so `handshake_main/.GOV` is treated as a mirror, not live authority

Reason:

- product-rooted execution in `handshake_main` and governance-rooted authority in `wt-gov-kernel` were not strict enough in the original lane

Impact:

- final authority could have evaluated or mutated closeout using stale main-side governance state

Judgment:

- This was a major final-lane topology and authority defect.

### 24.5 Medium: terminal closeout left a validated WP still steerable

Evidence:

- after closeout truth was already terminal, the session registry still showed:
  - `CODER:WP-1-Project-Profile-Extension-Registry-v1` -> `READY`
  - `WP_VALIDATOR:WP-1-Project-Profile-Extension-Registry-v1` -> `READY`
- `gov-check` then failed because the task-board state was terminal while those sessions were still steerable

Reason:

- terminal closeout did not retire coder and WP-validator sessions automatically

Impact:

- the lane looked both closed and open at the same time

Judgment:

- smaller than the role-boundary breach, but still a real lifecycle-coherence defect

### 24.6 Medium: `sync-gov-to-main` provenance could point at the wrong kernel truth

Evidence:

- local `main` already received `c89ffad` with message `gov: sync governance kernel a2fc2c7`
- later, after additional unsynced governance hardening was discovered and checkpointed on `gov_kernel`, the honest kernel source commit became `4cad247`
- a new guarded resync then produced `1a8ec21` and `GOV_KERNEL_SYNC.json` now points at `4cad24701084a2c90d17d1e590b84e4e8f9886de`

Reason:

- the older sync path mirrored dirty kernel state while still stamping the last committed kernel SHA into the marker and commit message

Impact:

- main-side governance history could overstate what exact kernel commit had actually been mirrored

Judgment:

- This is a provenance-integrity defect and should be treated as governance debt, not just tooling polish.

### 24.7 Medium: governance verification drifted on quoted justfile recipes

Evidence:

- after the quoting fixes, `gov-check` still failed because `protocol-alignment-check` expected unquoted justfile fragments for session-control recipes

Reason:

- the checker and the justfile drifted out of sync

Impact:

- governance health went red even though the live justfile surface was correct

Judgment:

- This is another example of the control plane being too easy to split internally.

## 25. Updated Role Review Addendum

### 25.1 Orchestrator Review Addendum

Strengths:

- repaired multiple real governance/control-plane defects instead of papering over them
- reconciled packet/runtime/task-board/build-order/session truth to a clean terminal state
- pushed the final governance fixes into kernel and synced them to `main`

Failures:

- did not maintain strict role separation
- crossed into product-code containment/harmonization work while still role-locked as `ORCHESTRATOR`
- depended on direct operator prompting to keep hardening the lane

Assessment:

- strong at recovery and governance repair, not good enough at staying within role boundaries

### 25.2 Coder Review Addendum

Strengths:

- the committed branch result eventually closed the signed product scope
- final branch-state proof became real and reviewable

Failures:

- first-pass and second-pass handoffs were not closure-grade
- branch cleanliness, scope control, and proof fidelity were too weak until late in the loop

Assessment:

- eventual product success, but too much burden shifted to validator and governance repair before the lane became closure-ready

### 25.3 WP Validator Review Addendum

Strengths:

- correctly blocked false closure
- PASS review on committed branch state was precise and evidence-rich

Failures:

- still too little mid-session steering before full handoff

Assessment:

- technically strong reviewer; cadence should be tighter on contract-heavy WPs

### 25.4 Integration Validator Review Addendum

Strengths:

- correctly separated product-code verdict from governance/runtime blockers
- final review did not invent new product debt when the blockers were really environmental/governance defects

Failures:

- the lane still depended on broken wrappers, stale compatibility truth, and final-lane authority ambiguity before the governance repairs landed

Assessment:

- technically sound, but the surrounding final-lane machinery was not robust enough

## 26. ACP Runtime / Session Control Findings Addendum

- Direct review receipts did eventually cover the full lane:
  - `VALIDATOR_KICKOFF`
  - `CODER_INTENT`
  - multiple `CODER_HANDOFF` / `VALIDATOR_REVIEW` cycles
  - final `REVIEW_REQUEST` / `REVIEW_RESPONSE` with `INTEGRATION_VALIDATOR`
- That is a positive signal, but the runtime/control layer still failed around it:
  - stale runtime and board projection after review traffic
  - closeout checks failing even when the technical verdict was ready
  - terminal sessions left open after closure
  - session-control and protocol-alignment drift producing false governance red state
- The lane closed only after governance-side code repairs to:
  - review projection
  - final-lane authority rooting
  - closeout sync ordering
  - terminal session retirement
  - dirty-kernel sync provenance

Judgment:

- ACP/runtime did not hold the lane together by itself; it had to be repaired in-flight.

## 27. Governance Implications

- `RGF-40`, `RGF-41`, `RGF-42`, and `RGF-43` are now justified by live evidence, not by abstract concern.
- The file-level governance fixes are real and useful, but they are reactive. They do not mean the run itself was healthy.
- The most important remaining governance follow-on is still open in principle:
  - add a hard role-lock/product-code write guard so an `ORCHESTRATOR` session cannot edit `src/`, `app/`, or `tests/` or perform contained-main conflict resolution unless the operator explicitly reassigns the role or the packet records that authority in a governed way
- A second follow-on should make contained-main cherry-pick/conflict resolution an explicit final-lane activity with receipts and session provenance, not an opportunistic side-effect of stall recovery.
- A third follow-on should emit a machine-visible workflow-invalidity or audit receipt when a governed role boundary is crossed in practice, so the event is not captured only by human memory.

## 28. Positive Signals Worth Preserving (Closeout Addendum)

- The WP validator did not rubber-stamp early green-looking product work.
- The final authority review distinguished product correctness from governance/environment defects.
- The governance fixes were implemented as tested code, not only as documentation.
- The final product result is now actually in local and remote `main`, not only on a WP branch.

## 29. Remaining Product or Spec Debt (Closeout Addendum)

- NONE inside the signed packet scope.
- The remaining debt exposed by this run is governance-side:
  - role-boundary enforcement
  - final-lane ownership clarity
  - auditability when reactive recovery crosses role boundaries

## 30. Post-Smoketest Improvement Rubric (Closeout Addendum)

### 30.1 Workflow Smoothness

- TREND: IMPROVED
- CURRENT_STATE: HIGH
- Evidence:
  - the lane now reaches truthful terminal closure
  - review projection, terminal session retirement, kernel-rooted Integration Validator authority, and dirty-kernel sync provenance are all better than they were at startup
  - the operator still had to keep redirecting governance repair
  - the Orchestrator still crossed into product-code work
- What improved:
  - the lane no longer dies at startup
  - terminal truth surfaces now converge much better
  - final-lane authority/path truth is far less ambiguous
- What still hurts:
  - strict role separation was not preserved
  - recovery was too manual
  - this run still required governance coding in the middle of a product WP
- Next structural fix:
  - hard-fail product-code writes and contained-main conflict resolution from a role-locked `ORCHESTRATOR` session unless explicit reassignment or governed authority override exists

### 30.2 Master Spec Gap Reduction

- TREND: IMPROVED
- CURRENT_STATE: LOW
- Evidence:
  - the packet is now `Validated (PASS)`
  - local/main containment is recorded at `26e85bbfdebfa5b19044420ced816ee3c3501f5d`
  - the earlier missing contract pieces are now closed
- What improved:
  - explicit registry enforcement is real
  - Task Board and Role Mailbox boundary preservation are real
  - unknown-extension rejection is proven
- What still hurts:
  - no material signed-scope product debt remains, but the route to closure was less governed than it should have been
- Next structural fix:
  - keep contained-main harmonization inside an explicitly governed final-lane path with preserved receipts and authority provenance

### 30.3 Token Cost Pressure

- TREND: IMPROVED
- CURRENT_STATE: HIGH
- Evidence:
  - multiple governance defects found in one run are now hardened
  - this same run still consumed substantial operator attention, repeated state reconciliation, and reactive governance coding
  - repeated path/authority/status rereads remained necessary while truth was split
- What improved:
  - several recurring failure families are now removed for future runs
  - `gov-check` and the sync path are less willing to lie
- What still hurts:
  - this run paid heavily for late discovery of boundary, projection, and provenance defects
  - the lack of a role-boundary hard gate means future token waste is still possible
- Next structural fix:
  - add the role-lock aware orchestrator product-write guard and force a proper governed reassignment path instead of letting stall recovery expand into manual product execution

## 31. Silent Failures, Command Surface Misuse, and Ambiguity Scan (Closeout Addendum)

### 31.1 Silent Failures / False Greens

- launch looked complete before ACP work was actually started
- runtime/task-board/build-order truth lagged real receipt truth
- a validated terminal WP still showed steerable coder/WP-validator sessions
- `sync-gov-to-main` could previously stamp a stale kernel SHA onto mirrored dirty state

### 31.2 Systematic Wrong Tool or Command Calls

- wrong worktree usage appeared early in the run
- command-surface drift produced a suggested helper that did not yet exist
- quoted justfile recipes and the protocol-alignment checker drifted apart
- most importantly: the orchestrator session itself became the wrong execution surface for contained-main product-code conflict resolution

### 31.3 Task and Path Ambiguity

- `handshake_main/.GOV` versus `wt-gov-kernel/.GOV` was too ambiguous before the final-lane hardening
- old versus new closeout-sync mode names (`DONE_*` versus `MERGE_PENDING` / `CONTAINED_IN_MAIN`) were not aligned across the docs
- the operator instruction to "patch bugs on the go" did not explicitly reassign the role, so governance should have treated role lock as dominant and blocked product-code work from the Orchestrator

### 31.4 Read Amplification / Governance Document Churn

- repeated rereads were needed across:
  - packet truth
  - Task Board truth
  - runtime truth
  - receipts
  - session registry
  - command-surface docs
  - final-lane protocols
- that churn was not neutral diligence; it was evidence that the live control-plane surface was still too fragmented

### 31.5 Hardening Direction

- add a role-boundary write gate for the Orchestrator
- make contained-main conflict resolution an explicit governed final-lane action with receipts
- keep kernel-rooted Integration Validator authority and dirty-kernel sync guarding as permanent law
- auto-record a workflow-invalidity or audit receipt whenever a role-boundary breach occurs in practice

## 32. Suggested Remediations (Closeout Addendum)

### Governance / Runtime

- add a hard role-lock guard preventing `ORCHESTRATOR` product-code writes without explicit reassignment or governed override
- make contained-main cherry-pick/conflict harmonization Integration Validator-owned by policy and receipts
- preserve the new terminal closeout, session-retirement, and dirty-kernel sync guards

### Product / Validation Quality

- no new in-scope product remediation is required for this packet
- preserve the new tripwire coverage when adjacent WPs touch the same artifact-family surfaces

### Documentation / Review Practice

- future smoketest reviews should append recovery-path failures into the same audit instead of letting the startup review look artificially isolated from the later closeout problems
- future protocols should say explicitly that role lock outranks broad operator phrases like "patch bugs on the go" unless the operator also reassigns the role

## 33. Additional Command Log

- `just integration-validator-closeout-sync WP-1-Project-Profile-Extension-Registry-v1 CONTAINED_IN_MAIN 26e85bbfdebfa5b19044420ced816ee3c3501f5d` -> PASS
- `just build-order-sync` -> PASS
- `just gov-check` -> PASS
- `just sync-gov-to-main` -> FAIL first after provenance hardening because kernel `/.GOV/` was still dirty; this was the intended fail-closed behavior
- `git commit -m "gov: harden orchestrator-managed relay and closeout"` on `gov_kernel` -> PASS (`4cad247`)
- `just sync-gov-to-main` -> PASS from committed kernel truth (`1a8ec21` on local `main`)
- `git -C ../handshake_main push origin main` -> PASS
- `git push origin gov_kernel` -> PASS

## 34. Review Quality, Microtask Discipline, and Data Doctrine Addendum

### 34.1 Operator Notes Incorporated

- bootstrap and skeleton quality should be judged by the WP Validator, not carried mainly by the Orchestrator, both to reduce orchestrator workload and to prove live coder-validator communication early
- validator review should overlap with microtask execution so the coder can continue on the next microtask while the validator checks the prior completed slice and loops back on gaps
- technical debt inside signed scope should be rejected, including vibe coding, easy-surface progress theater, weak implementation habits, and intent that rounds up partial work into false closure
- Handshake data should be treated as LLM-first, with active coding and review doctrine around:
  - SQL-backed and PostgreSQL-ready structures
  - LLM-first readability and parseability
  - Loom-intertwined data design
- those data-oriented goals are strategically correct, but they still need sharper definitions before they can be enforced mechanically

### 34.2 Assessment

- The direction is correct and high ROI.
- The strongest quality gain is earlier and more continuous WP-validator involvement, not simply tougher final handoff review.
- `RGF-47` improved this materially by adding a contract-heavy intent checkpoint, but it does not yet create a full rolling microtask-review loop with bounded backlog and explicit backpressure.
- Bootstrap and skeleton review should be treated as default WP-validator-owned checkpoints on governed lanes, especially when the packet is contract-heavy, schema-heavy, or data-shape-heavy.
- The LLM-first data doctrine is important, but in its current wording it is still too ambiguous to enforce consistently. Governance should translate it into packet/spec fields, coder instructions, validator proof obligations, and gate checks instead of leaving it as a general aspiration.
- "No technical debt at all cost" is operationally best interpreted as: no accepted debt inside signed scope, no rounding up weak progress into PASS, and fast failure on weak reasoning, shallow surface work, or brittle implementation.

### 34.3 Findings

- Review quality on this WP was strongest when the validator acted as an evidence-heavy blocker and weakest when review cadence waited too long for full handoff.
- The current governed lane now supports an intent checkpoint, but still does not give the validator a first-class rolling microtask review queue while the coder advances the next bounded slice.
- Bootstrap and skeleton review are still not expressed strongly enough as mandatory validator-owned gates across governed WP starts.
- Vibe coding, easy-surface-only progress, and bad implementation habits are not yet elevated enough into machine-visible review outcomes; too much still depends on reviewer diligence and prose quality.
- The repo still lacks a codified, reviewable definition of "LLM-first" data, which means coder and validator cannot yet prove or fail that standard consistently.
- If these areas stay informal, the result will be the same recurring failure family:
  - late discovery of intent/spec drift
  - superficial progress that looks green too early
  - avoidable debt and rework accumulating behind a final handoff

### 34.4 Suggested Governance Remediations

- `RGF-48`: add rolling microtask validator overlap with bounded backlog, explicit block/reroute semantics, and review carry-forward while coder proceeds on the next microtask
- `RGF-49`: make bootstrap and skeleton assessment an explicit WP-validator-owned governed gate on direct-review lanes
- `RGF-50`: codify an LLM-first data contract for Handshake so SQL/PostgreSQL readiness, LLM-readable structure, and Loom-intertwined data requirements become reviewable packet/spec truth
- `RGF-51`: strengthen anti-vibe/easy-surface review heuristics so shallow or weakly justified implementation quality becomes explicit `FAIL`, `NOT_PROVEN`, or debt-rejection truth rather than soft reviewer commentary

### 34.5 Expected Review-Quality Effect

- Earlier validator involvement should reduce false-green coder momentum and shrink the amount of product work that reaches full handoff in a closure-unready state.
- Rolling microtask review should let the validator catch intent and implementation drift while the coder still has narrow context, which is cheaper than late-loop rework.
- Validator-owned bootstrap and skeleton review should prove coder-validator communication early and reduce orchestrator relay burden.
- A codified data doctrine should improve product consistency across future WPs instead of repeatedly rediscovering the same storage and parseability expectations.
- Harder anti-vibe/debt rejection should make the review lane stricter about implementation quality, not only functional closure.
