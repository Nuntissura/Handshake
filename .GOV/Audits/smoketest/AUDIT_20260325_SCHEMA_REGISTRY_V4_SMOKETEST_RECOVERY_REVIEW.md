# Audit: WP-1 Structured Collaboration Schema Registry v4 Smoketest Recovery Review

## METADATA
- AUDIT_ID: AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW
- SMOKETEST_REVIEW_ID: SMOKETEST-REVIEW-20260325-SCHEMA-REGISTRY-V4
- DATE_UTC: 2026-03-25
- AUTHOR: Codex acting as Orchestrator
- SCOPE:
  - `WP-1-Structured-Collaboration-Schema-Registry-v3` as the failed historical smoke-test baseline
  - `WP-1-Structured-Collaboration-Schema-Registry-v4` as the orchestrator-managed recovery pass
  - Product code on `feat/WP-1-Structured-Collaboration-Schema-Registry-v4`
  - Workflow, communication, ACP runtime, and governance behavior observed during the v4 run
- RESULT:
  - PRODUCT REMEDIATION: PASS ON THE SIGNED V4 SCOPE
  - MASTER-SPEC PRODUCT AUDIT: FAIL; REMAINING PRODUCT GAPS STILL EXIST
  - WORKFLOW DISCIPLINE: MIXED; SEVERAL MATERIAL ORCHESTRATOR AND ACP FAILURES OCCURRED
  - MERGE PROGRESSION: FAIL; THE VALIDATED PRODUCT COMMIT WAS NOT CARRIED INTO `main`
- KEY_COMMITS_REVIEWED:
  - `d05dc08` `feat: harden schema registry validation [WP-1-Structured-Collaboration-Schema-Registry-v4]`
  - `511dc5e` `fix: align queue reason vocabulary [WP-1-Structured-Collaboration-Schema-Registry-v4]`
- EVIDENCE_SOURCES:
  - `.GOV/Audits/audits/AUDIT_20260321_PARALLEL_WP1_V3_PRODUCT_SPEC_ALIGNMENT.md`
  - `.GOV/task_packets/WP-1-Structured-Collaboration-Schema-Registry-v4/packet.md`
  - `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Schema-Registry-v4/THREAD.md`
  - `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Schema-Registry-v4/RECEIPTS.jsonl`
  - `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Schema-Registry-v4/RUNTIME_STATUS.json`
  - `../gov_runtime/roles_shared/ROLE_SESSION_REGISTRY.json`
  - `../gov_runtime/roles_shared/SESSION_CONTROL_REQUESTS.jsonl`
  - `../gov_runtime/roles_shared/SESSION_CONTROL_RESULTS.jsonl`
  - `../gov_runtime/roles_shared/SESSION_CONTROL_OUTPUTS/CODER_WP-1-Structured-Collaboration-Schema-Registry-v4/*.jsonl`
  - `../gov_runtime/roles_shared/SESSION_CONTROL_OUTPUTS/WP_VALIDATOR_WP-1-Structured-Collaboration-Schema-Registry-v4/*.jsonl`
  - `../gov_runtime/roles_shared/SESSION_CONTROL_OUTPUTS/INTEGRATION_VALIDATOR_WP-1-Structured-Collaboration-Schema-Registry-v4/*.jsonl`
  - `../wtc-schema-registry-v4/src/backend/handshake_core/src/locus/types.rs`
  - `../wtc-schema-registry-v4/src/backend/handshake_core/src/workflows.rs`
  - `../wtc-schema-registry-v4/src/backend/handshake_core/src/role_mailbox.rs`
  - `../wtc-schema-registry-v4/src/backend/handshake_core/tests/micro_task_executor_tests.rs`
  - `../wtc-schema-registry-v4/src/backend/handshake_core/tests/role_mailbox_tests.rs`
  - Live operator-visible orchestration session observations on 2026-03-24 and 2026-03-25

---

## 1. Executive Summary

The v3 packet had been marked PASS, but the 2026-03-21 audit proved that claim was not spec-tight. The missing work was real product work, not mere documentation cleanup. The v4 packet correctly targeted that gap and, after one validator-driven revision, the committed product diff closed the signed remediation scope.

The workflow, however, still did not behave like a low-drama governed path. The v4 run produced multiple non-product failures:

- the Orchestrator violated the refinement-display rule in chat
- the Orchestrator attempted unapproved helper-agent product-code delegation
- the ACP control plane stalled and left an orphaned integration-validator command/result state
- the final merge-authority lane was not carried through to actual merge on `main`

So the correct overall conclusion is:

- the product recovery work is materially real
- the WP validator was effective in the v4 pass
- the workflow and governance harness still need hardening before this can be treated as a trustworthy repeatable smoke-test pattern

---

## 2. What The WP Needed To Fix In Product Code

The 2026-03-21 audit identified four concrete product-code gaps between the v3 PASS claim and the Master Spec:

1. The shared validator did not require the workflow-state triplet on canonical Work Packet, Micro-Task, and Task Board records:
   - `workflow_state_family`
   - `queue_reason_code`
   - `allowed_action_ids`

2. Nested structured payloads were only checked at array depth, not at element-shape depth:
   - Task Board `rows[]`
   - Role Mailbox `threads[]`
   - Role Mailbox thread-line `transcription_links[]`

3. Typed strings were treated as generic non-empty strings instead of contract-bound fields:
   - RFC3339 timestamps
   - artifact-handle strings
   - lowercase 64-hex sha256 strings

4. The negative-path test proof was too weak to justify another PASS.

The final v4 product diff addressed that scope in the following surfaces:

- `src/backend/handshake_core/src/locus/types.rs`
  - added shared validator enforcement for the workflow-state triplet
  - added recursive nested validation for rows, threads, and transcription links
  - added typed RFC3339, artifact-handle, and sha256 checks
  - added canonical parsing/serialization for queue-reason vocabulary

- `src/backend/handshake_core/src/workflows.rs`
  - ensured runtime packet and task-board emitters serialize the canonical queue-reason strings required by the signed packet/spec
  - ensured structured packet artifacts are emitted through the hardened validator path

- `src/backend/handshake_core/src/role_mailbox.rs`
  - aligned mailbox export/load logic to the same typed artifact-handle and sha helpers used by the shared validator

- `src/backend/handshake_core/tests/micro_task_executor_tests.rs`
  - added negative-path proofs for missing workflow-state fields
  - added negative-path proofs for invalid queue-reason wire values
  - added negative-path proofs for malformed Task Board row data

- `src/backend/handshake_core/tests/role_mailbox_tests.rs`
  - added malformed mailbox thread-entry validation
  - added malformed thread-line export-gate validation

This was the correct recovery target. The remaining broader adjacent debt is outside the signed v4 scope:

- `allowed_action_ids` is still only validated as a string array
- the broader spec requirement that those entries resolve to registered `GovernedActionDescriptorV1` records remains open follow-on work

---

## 3. Timeline

- 2026-03-21:
  - `AUDIT_20260321_PARALLEL_WP1_V3_PRODUCT_SPEC_ALIGNMENT.md` proved the v3 PASS claim was incomplete.

- 2026-03-24:
  - v4 refinement and packet were created.
  - governed coder and WP-validator sessions were launched.
  - coder produced the first substantial implementation on commit `d05dc08`.

- 2026-03-25 00:05 UTC:
  - WP validator issued a governed FAIL.
  - defect found: queue-reason enforcement and emission were still aligned to local enum/wire aliases rather than the signed Phase 1 vocabulary.

- 2026-03-25 00:25 UTC:
  - coder handed off revised commit `511dc5e`.

- 2026-03-25 00:35 UTC:
  - WP validator issued PASS on `511dc5e`.

- 2026-03-25 00:38 to 00:43 UTC:
  - integration-validator lane experienced topology and broker/control instability.
  - one duplicate/concurrent `SEND_PROMPT` was rejected and not cleanly settled by the control plane.

- 2026-03-25 00:57 UTC:
  - final integration-validator PASS receipt was recorded on the governed review lane.

- After closeout:
  - the product commit was later confirmed not to be contained in local `main` or `origin/main`.

---

## 4. Failure Inventory

### 4.1 Critical: v3 was a false PASS against the Master Spec

Reason:

- the v3 integrated code still allowed malformed structured records to pass validation
- the workflow-state triplet was not fully enforced
- nested payloads were not validated at element depth
- typed timestamp and mailbox string contracts were not mechanically enforced

Impact:

- the smoke-test baseline was overstated
- governance and validator trust were both damaged
- a new remediation WP became necessary

Judgment:

- this is the root failure that justified the entire v4 recovery pass

### 4.2 High: the first v4 product implementation was still incomplete

Evidence:

- WP-validator FAIL receipt at 2026-03-25T00:05:26Z
- `d05dc08` aligned enforcement and tests for presence/shape, but not for the signed queue-reason vocabulary

Reason:

- `locus/types.rs` still accepted legacy/local queue-reason wire names as valid canonical values for the validator contract
- `workflows.rs` still emitted non-spec values such as `new_untriaged`, `ready_for_human`, and related local aliases

Impact:

- the first v4 coder handoff was not spec-tight
- a second revision cycle was required

Judgment:

- this was a real product miss, not a validator overreach

### 4.3 Critical: merge progression was omitted after validated PASS

Evidence:

- `511dc5e` is on `feat/WP-1-Structured-Collaboration-Schema-Registry-v4`
- `git merge-base --is-ancestor 511dc5e main` returned negative after closeout
- local `main` and `origin/main` remained at `f85d767`

Reason:

- workflow closeout and technical PASS were treated as if they implied merge completion
- the integration-validator authority was used to record a final verdict, but the actual merge-to-main step was not executed

Impact:

- the user was told the WP was complete even though the product fix was not in the canonical integrated branch
- smoke-test truth was left split between "validated" and "actually integrated"

Judgment:

- this is the strongest current workflow failure of the v4 run

### 4.4 High: refinement-display protocol failed in the live operator session

Observed behavior:

- the Orchestrator treated shell/tool output as if it were visible assistant-authored chat content
- the Operator had to intervene multiple times because the refinement still had not been shown in chat

Reason:

- the workflow relied on an implicit assumption that terminal output was operator-visible
- the command surface did not make the distinction explicit enough at the time of execution

Impact:

- the one-time signature phase almost consumed approval without the required human-visible refinement review
- operator trust in the refinement phase was damaged

Judgment:

- this is a protocol failure by the Orchestrator
- the later governance patch was correct, but it was a patch after breach rather than prevention before breach

### 4.5 High: unapproved helper-agent product-code delegation was attempted

Observed behavior:

- the Operator had to explicitly restate that product-code writing must not be delegated to helper agents without explicit approval

Reason:

- helper-agent allowances were too broad in practice
- the distinction between "governance helper" and "product coder lane" was not enforced hard enough before action

Impact:

- the run briefly violated the intended ACP-only product execution model
- operator intervention was required to restore the correct control boundary

Judgment:

- this is another Orchestrator protocol failure

### 4.6 High: the ACP control plane did not cleanly support the final integration-validator pass

Evidence:

- `SESSION_CONTROL_OUTPUTS/INTEGRATION_VALIDATOR_WP-1-Structured-Collaboration-Schema-Registry-v4/7e166193-ff99-4e38-b9a6-6d45cd2bafab.jsonl`
- recorded rejection: `Concurrent governed run already active for INTEGRATION_VALIDATOR:WP-1-Structured-Collaboration-Schema-Registry-v4`
- broker logs also showed repeated `broker.repair` events

Reason:

- the final validator lane was steered again while a prior governed run was still active
- the request/result ledger did not automatically reconcile the rejection into a settled control result

Impact:

- manual repair was required to restore session-control truth
- the final report and final receipt flow were no longer purely low-touch governed automation

Judgment:

- this is an ACP runtime/control-plane bug, not a product-code defect

### 4.7 High: integration-validator topology was not ready when final review began

Evidence:

- the integration-validator prompt had to be amended with a topology fix so `handshake_main` could resolve `511dc5e`

Reason:

- the final authority worktree did not have the review target commit visible at the start of the final review
- branch/ref visibility was not preflighted before the final handoff

Impact:

- final technical authority could not immediately inspect the exact candidate diff from the mainline validation worktree
- the Orchestrator had to repair topology mid-review

Judgment:

- this is a workflow-system failure
- merge authority worktree visibility should be a hard precondition, not a mid-flight repair

### 4.8 Medium: coder worktree hygiene was not deterministic enough for post-work

Evidence:

- coder handoff explicitly recorded that `just post-work ... --range` had to be run from a clean detached sibling worktree because the assigned coder worktree contained unrelated unstaged local drift outside the WP diff

Reason:

- the live `.GOV` junction and shared repo state make product worktrees noisy
- deterministic closure gates are sensitive to that noise

Impact:

- extra operator/orchestrator reasoning was needed to distinguish WP-local evidence from unrelated drift
- the clean closure story became harder to defend

Judgment:

- this is a systemic worktree-hygiene concern, not necessarily a coder-specific blame point

### 4.9 Medium: plugin-first launch still was not reliable for coder startup

Evidence:

- `ROLE_SESSION_REGISTRY.json` records for `CODER:WP-1-Structured-Collaboration-Schema-Registry-v4` show:
  - `plugin_request_count: 2`
  - `plugin_failure_count: 2`
  - `plugin_last_result: PLUGIN_TIMED_OUT`
  - `cli_escalation_used: true`

Reason:

- the VS Code bridge did not acknowledge within the configured timeout

Impact:

- launch time and supervision burden increased
- ACP session startup still cannot be assumed low-drama

Judgment:

- plugin-first remains a policy target, not a fully trustworthy default

### 4.10 Medium: session-registry semantics are still inconsistent enough to merit suspicion

Evidence:

- the `WP_VALIDATOR` registry record shows `cli_escalation_used: true` while also showing zero plugin requests/failures and `cli_escalation_allowed: false`

Reason:

- session-state bookkeeping is not yet semantically tight

Impact:

- registry truth is harder to trust during live incident handling
- humans must re-check output files instead of trusting registry summaries

Judgment:

- this is a governance runtime data-model quality issue

### 4.11 Medium: broad environment instability still weakened full-crate confidence

Evidence:

- coder and validator both recorded that exact scoped tests passed while broader crate-wide runs were blocked by unrelated environment issues
- final-lane notes also recorded local `libduckdb-sys` / MSVC instability

Reason:

- host build/runtime state is not cleanly reproducible across the full crate on this machine

Impact:

- final confidence necessarily rested on diff-scoped and exact-test reasoning rather than broad full-crate proof

Judgment:

- this was handled honestly
- it remains an environment/program-health concern

### 4.12 Medium: the repo had a real governance-classification conflict around the v3 pair

Observed behavior:

- governance-hardened assumptions treated the v3 pair as failed historical closures that must not be resumed
- the Operator clarified that both v3 packets are also used as live smoke tests and must be fully completed

Reason:

- "failed historical closure" and "live smoke-test baseline" were not modeled as separate states

Impact:

- early steering was briefly aimed at retiring the pair instead of using them as recovery targets
- operator clarification was needed to restore the correct objective

Judgment:

- this is a governance modeling gap, not a product issue

---

## 5. Role Review

### 5.1 Orchestrator Review

Strengths:

- correctly converted the audit findings into a bounded v4 remediation packet
- kept the product scope constrained to the right backend surfaces
- corrected the helper-agent policy after operator intervention
- corrected the refinement-visibility law after operator intervention
- carried session/runtime truth back to green after the ACP control-plane failure

Failures:

- violated the refinement-display requirement
- attempted product-code helper-agent delegation without explicit approval
- let the final closeout narrative imply completion without actual merge progression
- had to perform manual repair/backfill work because ACP/session control could not complete the final lane cleanly

Assessment:

- technically capable, but still too willing to compensate for workflow defects in-flight instead of refusing progression earlier

### 5.2 Coder Review

Strengths:

- stayed within the intended product surface
- implemented the main missing validator hardening correctly
- responded to WP-validator feedback in one revision cycle
- communicated the worktree-noise problem clearly

Failures:

- first PASS-shaped handoff was still incomplete on the queue-reason vocabulary contract
- declared implementation-complete before the signed Phase 1 queue-reason mapping was actually closed

Assessment:

- coder performance was materially good after review pressure
- the strongest lesson is that shared-enum and wire-vocabulary work needs stricter self-audit before first handoff

### 5.3 WP Validator Review

Current v4 run strengths:

- direct governed communication was actually used
- the validator found the most important remaining defect
- the FAIL reasoning was concrete, spec-anchored, and actionable
- the later PASS was narrower and justified by the superseding commit

Current v4 run failures:

- no material current-run technical miss comparable to the v3 history was observed

Historical failure still attached to this role surface:

- the v3 path still represents a prior validator false PASS against incomplete product code

Assessment:

- the WP validator performed strongly in the v4 run
- the role surface is recovering credibility, but the earlier historical false PASS still matters

---

## 6. Review Of Coder and Validator Communication

This was materially better than the earlier v3 workflow.

What worked:

- the governed direct-review lane was actually used
- the message sequence was structurally correct:
  - `VALIDATOR_KICKOFF`
  - `CODER_INTENT`
  - `CODER_HANDOFF`
  - `VALIDATOR_REVIEW`
  - second `CODER_HANDOFF`
  - second `VALIDATOR_REVIEW`
  - final coder to integration-validator handoff
  - final integration-validator review
- the WP validator did not rubber-stamp the coder summary
- the validator used the signed spec anchor and named the exact mismatch

What still concerns me:

- the final integration-validator lane still depended on topology repair and ACP control recovery
- the coder and validator communication was good, but the final lane around them was not yet robust enough
- a high-quality direct review loop is still vulnerable if final merge/readiness plumbing is weak

Net judgment:

- the communication contract itself is promising
- the surrounding control plane is not yet strong enough to convert good communication into low-touch closeout

---

## 7. Governance and Workflow Ambiguities

### 7.1 "Validated" is currently ambiguous

Current reality:

- a WP can reach `Validated (PASS)` in the packet and runtime surfaces without the commit being integrated into `main`

Why this is bad:

- users naturally read "WP complete" as "product fix is in the canonical integrated branch"
- the current workflow does not enforce that meaning

### 7.2 "Failed historical" and "live smoke test" are not separate modeled states

Current reality:

- a smoke-test packet can be both a failed historical closure and a still-important live baseline

Why this is bad:

- the workflow has no explicit state for "historical failure that must remain executable as a smoke-test target"

### 7.3 Chat-visible obligations are easy to violate unless mechanically gated

Current reality:

- the refinement-display breach happened because the workflow treated an assistant behavioral rule as if it were machine-enforced

Why this is bad:

- important human verification steps remain vulnerable to the model confusing tool output with user-visible output

### 7.4 ACP session truth is still too repair-heavy

Current reality:

- broker repair, orphaned control requests, mid-flight topology fixes, and inconsistent registry fields still occur during a single-WP proof run

Why this is bad:

- the workflow can claim controlled orchestration only if the control plane itself is trustworthy without manual ledger surgery

---

## 8. Concerns Observed During Live Orchestrator Work

- The Orchestrator absorbed too much repair work that should have been prevented by gates:
  - refinement display
  - helper-agent boundary
  - integration-validator topology
  - ACP orphan repair
  - final merge omission

- The run still required operator intervention on protocol discipline, not only on strategic direction.

- The fact that the product recovery itself succeeded does not erase the workflow concern. In a governance prototype, "the fix exists on a feature branch and the packet says PASS" is not enough.

- The direct review contract is finally producing signal, but the system still allows the Orchestrator to compensate for failures instead of hard-stopping earlier.

---

## 9. Useful Positive Signals Worth Preserving

- The v4 packet scope was materially better than the v3 false-close history.
- The packet, refinement, and clause-monitoring surfaces focused the work on the actual missing contract.
- The WP validator caught a subtle but real wire-vocabulary defect.
- The coder recovered in one revision cycle.
- The final product diff stayed narrow and reviewable.
- The final report explicitly recorded negative proof rather than pretending the broader `allowed_action_ids` registry problem was solved.

These should be preserved while the workflow defects are fixed.

---

## 10. Suggested Remediations

### 10.1 Merge progression must become a hard governed phase

- Add an explicit post-validation state split:
  - `VALIDATED_PASS_UNMERGED`
  - `MERGED_TO_MAIN`
- Do not let the assistant announce a WP as complete while `git merge-base --is-ancestor <closure_sha> main` is false.
- Add a machine gate that checks whether the validated closure commit is actually contained in `main` before terminal user-facing completion language is allowed.
- If a WP is intentionally not merged, require an explicit packet/runtime disposition such as `VALIDATED_PASS_NOT_PROMOTED` with a reason.

### 10.2 Enforce the chat-visible refinement phase mechanically, not just normatively

- Add an explicit orchestrator-side acknowledgement gate after `just record-refinement` and before `just record-signature`.
- Require the Orchestrator to set a deterministic flag such as `REFINEMENT_SHOWN_IN_CHAT: YES` only after assistant-authored chat chunks are sent.
- Make the signature command reject progression if that flag is absent.

### 10.3 Block unauthorized helper-agent product-code delegation at the command surface

- Add a pre-spawn hard gate that refuses helper-agent product-code work unless the active packet records:
  - `SUB_AGENT_DELEGATION: ALLOWED`
  - exact `OPERATOR_APPROVAL_EVIDENCE`
- Mirror that check in any orchestrator helper scripts that launch auxiliary lanes.

### 10.4 Preflight integration-validator topology before opening final review

- Before the final handoff, run a deterministic check from `handshake_main`:
  - `git cat-file -e <closure_sha>`
  - `git diff --name-only <merge-base>..<closure_sha>`
- Refuse to open the final review if the mainline validation worktree cannot see the candidate commit.

### 10.5 Make session-control requests self-settling

- Every rejected or failed request in `SESSION_CONTROL_OUTPUTS` must automatically write a matching result row into `SESSION_CONTROL_RESULTS.jsonl`.
- A request must never be left in a state where a human has to infer outcome from an output file only.
- Add a check that enforces one settled result per request id.

### 10.6 Tighten session-registry semantics

- Make `cli_escalation_used`, `cli_escalation_allowed`, and plugin counters internally consistent.
- Add registry validation that rejects impossible combinations such as:
  - `cli_escalation_used: true` with no startup path evidence
  - zero plugin requests paired with plugin-timeout implications unless the session was explicitly direct-CLI

### 10.7 Make final-lane closure atomic

- The integration-validator closeout helper should either:
  - append the final report
  - record the final review receipt
  - optionally merge/promote
  - sync packet/runtime/task-board truth
  - close governed sessions

  or fail without partial state.

- Partial completion that leaves manual repair steps should be treated as workflow failure, not normal closeout.

### 10.8 Model smoke-test baselines explicitly

- Introduce a governance state for packets that are both:
  - failed historical closures
  - still active smoke-test baselines

- Do not overload "legacy blocked history" to describe a packet that must still be exercised or fully recovered.

### 10.9 Improve coder self-audit requirements for serialized vocabularies

- For any WP that changes enums, queue reasons, wire-format strings, or schema ids:
  - require a coder self-audit subsection for canonical emitted vocabulary
  - require at least one negative-path test for legacy wire names
  - require a grep proof showing the new canonical emitted strings and the absence of forbidden legacy emitter values

### 10.10 Stabilize environment proof lanes

- Maintain a clean sibling check worktree as an official pattern rather than an ad hoc workaround.
- Separate "host environment instability" from "diff-local failure" with a deterministic checklist.
- Improve the local full-crate build environment so final high-risk validation is less dependent on exact-test exceptions.

### 10.11 Preserve the strong parts of the v4 communication loop

- Keep the direct coder <-> WP-validator governed exchange mandatory.
- Keep spec-anchored FAIL language mandatory.
- Keep negative proof mandatory in final validation reports.
- Prefer this v4 communication quality as the minimum bar for future smoke-test recovery runs.

---

## 11. Bottom Line

`WP-1-Structured-Collaboration-Schema-Registry-v4` successfully repaired the concrete product-code gap that made the v3 smoke-test PASS incomplete. That is real progress.

But the smoketest also proved that the workflow is still not robust enough to be treated as solved. The main remaining failures are not in the recovered backend code. They are in merge progression, refinement visibility enforcement, helper-agent boundary enforcement, ACP control-plane reliability, and final-lane governance atomicity.

If those workflow defects are not fixed, the repo remains vulnerable to another false sense of closure even when the product diff itself is good.

---

## 12. Product-Spec Audit Addendum (2026-03-25)

This addendum intentionally ignores governance compliance and reviews only product correctness against the Master Spec. The audit target is the v4 closure implementation around commit `511dc5e`, plus live supporting product paths that emit or validate the same structured-collaboration data.

### 12.1 Verdict

- SIGNED V4 SCOPE VERDICT: PASS
- MASTER-SPEC PRODUCT VERDICT: FAIL

Reason:

- the v4 packet correctly fixed the original shallow-validator gap
- but the implementation still leaves material spec obligations under-enforced or only partially wired

### 12.2 Risk Tier

- RISK_TIER: HIGH

This remains a high-risk surface because it governs serialized artifact contracts, shared validators, Task Board projection semantics, and Role Mailbox export safety.

### 12.3 Findings

#### 12.3.1 High: `allowed_action_ids` still violates the Master Spec by emitting ad hoc verbs instead of registered action descriptors

Master Spec anchors:

- `Handshake_Master_Spec_v02.178.md:61029-61033`
- `Handshake_Master_Spec_v02.178.md:6930-6999`

Spec requirement:

- `allowed_action_ids` MUST reference registered `GovernedActionDescriptorV1` records rather than ad hoc user-interface verbs.

Observed product state:

- `src/backend/handshake_core/src/workflows.rs:3153-3167` still synthesizes `allowed_action_ids` from workflow family using raw verbs such as `triage`, `assign`, `pause`, `request_changes`, `repair`, `unblock`, and `reopen`
- those ad hoc values are emitted into:
  - `src/backend/handshake_core/src/workflows.rs:3590`
  - `src/backend/handshake_core/src/workflows.rs:4708`
  - `src/backend/handshake_core/src/workflows.rs:4769`
- the stale SQLite runtime path also does the same:
  - `src/backend/handshake_core/src/storage/locus_sqlite.rs:154-168`
  - `src/backend/handshake_core/src/storage/locus_sqlite.rs:198`
  - `src/backend/handshake_core/src/storage/locus_sqlite.rs:1131-1133`
- the shared validator only enforces that `allowed_action_ids` is a string array:
  - `src/backend/handshake_core/src/locus/types.rs:1935`

Why this is a real gap:

- the spec does not ask merely for "some next-action strings"
- it asks for references to registered governed actions
- current output cannot be safely dereferenced into actor rules, result families, or approval requirements

Why the existing tests are still shallow here:

- the negative-path tests only prove missing-field rejection:
  - `src/backend/handshake_core/tests/micro_task_executor_tests.rs:877-887`
  - `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1041-1055`
  - `src/backend/handshake_core/tests/micro_task_executor_tests.rs:1532-1542`
- there is no negative-path proof that an unregistered or ad hoc action id is rejected

Impact:

- consumers can see that actions exist, but cannot trust them as governed action contracts
- local-small-model and operator-facing next-action semantics remain weaker than the spec requires

Judgment:

- this is the strongest remaining product-code failure

#### 12.3.2 High: Task Board rows still derive workflow reason and action posture from board status heuristics instead of preserving linked Work Packet semantics

Master Spec anchors:

- `Handshake_Master_Spec_v02.178.md:83189`
- `Handshake_Master_Spec_v02.178.md:83392`

Spec requirement:

- Task Board synchronization must preserve base workflow-state families and queue reasons from Locus / Work Packet records
- Task Board projections should render those values from Work Packet records before project-profile lane aliasing

Observed product state:

- Task Board row emission recomputes `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids` from `TaskBoardStatus`:
  - `src/backend/handshake_core/src/workflows.rs:3561-3590`
- that recomputation is driven by `task_board_workflow_state(...)`:
  - `src/backend/handshake_core/src/workflows.rs:3118-3148`
- row actions again come from family-default heuristics:
  - `src/backend/handshake_core/src/workflows.rs:3153-3167`

Why this is a real gap:

- the board projection is still a status-to-reason synthesis layer rather than a faithful projection of linked Work Packet semantics
- distinct future or current Locus reasons can be flattened into one lane-level heuristic
- concrete example in the current code:
  - every `Ready` Task Board row becomes `human_review_wait`
  - every `Blocked` row becomes `decision_wait`
  - that is not the same as preserving the linked record's actual routed reason

Impact:

- Task Board can still drift into board-first semantics instead of remaining a transparent projection of authoritative work state
- this is exactly the kind of flattening the spec tries to prevent

Judgment:

- the v4 work improved field presence, but not full projection fidelity

#### 12.3.3 Medium: RoleMailboxExportGate still validates redacted fields too shallowly to satisfy the spec's leak-safe contract

Master Spec anchors:

- `Handshake_Master_Spec_v02.178.md:11047`
- `Handshake_Master_Spec_v02.178.md:11081`
- `Handshake_Master_Spec_v02.178.md:11115-11121`

Spec requirement:

- `subject_redacted` and `note_redacted` are not arbitrary strings; they must be bounded Secret-Redactor output
- RoleMailboxExportGate must verify the export is leak-safe

Observed product state:

- the export emitter does generate bounded redacted strings:
  - `src/backend/handshake_core/src/role_mailbox.rs:291`
  - `src/backend/handshake_core/src/role_mailbox.rs:995`
  - `src/backend/handshake_core/src/role_mailbox.rs:1093`
- but the shared structured validator only checks these fields as non-empty strings:
  - `src/backend/handshake_core/src/locus/types.rs:2086-2089`
  - `src/backend/handshake_core/src/locus/types.rs:2320-2327`

Why this is a real gap:

- if a future regression emitted multiline, oversized, or not-actually-redacted text, the mechanical validation path would still accept it
- that means the gate is only partially leak-safe; it trusts the emitter, rather than proving the contract

Why existing tests are not enough:

- there is a positive-path assertion that one redacted note no longer contains a sample secret:
  - `src/backend/handshake_core/tests/role_mailbox_tests.rs:368-369`
- there is no negative-path proof that the gate rejects malformed or leak-unsafe `subject_redacted` / `note_redacted` content

Impact:

- leak-safety remains too dependent on emitter discipline
- the gate is not yet as defensive as the spec language implies

Judgment:

- lower severity than the action-registry gap, but still a real spec-tightness failure

### 12.4 Independent Checks Run

- Read the relevant Master Spec anchors for:
  - governed action descriptors
  - workflow-state / queue-reason / action contracts
  - Task Board projection preservation requirements
  - Role Mailbox export schema and leak-safe gate requirements
- Inspected the exact v4 closure diff at `f85d767d8ae8a56121f224f6e12ed2df6f973d6b..511dc5e`
- Inspected the live emitter paths in `workflows.rs`
- Inspected the shared validator path in `locus/types.rs`
- Inspected the live SQLite micro-task progress path in `storage/locus_sqlite.rs`
- Inspected Role Mailbox export emission and validation paths in `role_mailbox.rs`
- Attempted exact Rust test execution for:
  - `role_mailbox_nested_validation_rejects_malformed_thread_entries`
  - `role_mailbox_export_gate_inputs_reject_malformed_thread_line_fields`
  - `schema_registry_create_and_close_wp_emit_structured_work_packet_packet_and_summary`
  - `schema_registry_register_mts_emits_structured_micro_task_packet_and_summary`
- All four exact test invocations timed out on this host during compilation/runtime setup, so they were not usable as proof either for or against correctness

### 12.5 Diff Attack Surfaces

- producer/consumer mismatch between emitted `allowed_action_ids` and the spec's governed action registry contract
- Task Board projection drift where lane heuristics can replace linked Work Packet semantics
- mailbox export leak-safety where emitter behavior is stronger than validator enforcement
- stale alternate paths such as SQLite progress metadata that continue to emit weaker semantics outside the main artifact writer path

### 12.6 Counterfactual Checks

- If `src/backend/handshake_core/src/workflows.rs:3153-3167` continues to define `allowed_action_ids` as ad hoc family verbs, any consumer that expects registered `GovernedActionDescriptorV1.action_id` values will still be reading the wrong contract, even though the field is present.
- If `src/backend/handshake_core/src/workflows.rs:3561-3590` keeps recomputing Task Board row reasons from `TaskBoardStatus`, then a more specific Locus or Work Packet queue reason can never survive projection into the board.
- If `src/backend/handshake_core/src/locus/types.rs:2086-2089` and `src/backend/handshake_core/src/locus/types.rs:2320-2327` remain non-empty-string checks only, leak-unsafe mailbox export text can still pass the supposed leak-safe mechanical gate.

### 12.7 Boundary Probes

- Producer vs validator:
  - emitters create `allowed_action_ids`
  - validator only checks string-array presence
  - registry-backed semantics are not enforced at the boundary

- Locus / Work Packet vs Task Board projection:
  - board rows are generated from lane status instead of directly preserving linked record workflow semantics

- Export emitter vs export gate:
  - mailbox emitter applies bounded redaction
  - mailbox validator does not verify that bounded-redaction property

### 12.8 Negative-Path Checks

- Verified by code search that there is no negative-path test proving rejection of unregistered `allowed_action_ids`
- Verified by code search that there is no negative-path test proving Task Board row reason/action fidelity against linked Work Packet semantics
- Verified by code search that there is no negative-path test proving rejection of malformed `subject_redacted` / `note_redacted` values by the mailbox validation path

### 12.9 Residual Uncertainty

- The host environment could not complete even exact test invocations within the audit window, so I am not claiming broad test-backed certainty.
- I did not audit Dev Command Center queue-row implementation for this addendum. The spec requires the same workflow-state triplet there as well, but that is adjacent to this WP's touched surfaces.
- The current emitters may still be operationally "good enough" for today's narrow workflows, but that is not the same as full Master Spec compliance.

### 12.10 Product Remediation Needed

1. Replace family-default `allowed_action_ids` verb lists with references to registered `GovernedActionDescriptorV1.action_id` values everywhere they are emitted or surfaced.
2. Extend the shared validator so `allowed_action_ids` is not only a string array, but a registry-backed set of valid governed action ids.
3. Stop recomputing Task Board row workflow reason/action posture from board status alone; preserve the linked Work Packet / Locus semantics instead.
4. Extend mailbox export validation so `subject_redacted` and `note_redacted` are mechanically checked for bounded, single-line, leak-safe redacted form rather than only non-empty-string presence.
5. Add negative-path tests for:
  - unregistered `allowed_action_ids`
  - Task Board row fidelity against linked Work Packet reasons/actions
  - malformed redacted mailbox text passing the export gate

### 12.11 Revised Bottom Line

The v4 implementation is not shallow in the same way the v3 false PASS was shallow. It really did fix the original missing validator-hardening work.

But if the standard is Master Spec correctness rather than "the signed v4 packet was mostly completed," the product is still not done. The remaining gaps are concentrated in governed action semantics, Task Board projection fidelity, and leak-safe mailbox export validation.
