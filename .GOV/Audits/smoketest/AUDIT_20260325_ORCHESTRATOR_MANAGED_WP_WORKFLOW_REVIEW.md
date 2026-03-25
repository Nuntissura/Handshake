# Audit: Orchestrator-Managed WP Workflow Review

## METADATA
- AUDIT_ID: AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW
- SMOKETEST_REVIEW_ID: SMOKETEST-REVIEW-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW
- REVIEW_KIND: COMPARISON
- DATE_UTC: 2026-03-25
- AUTHOR: Codex acting as Orchestrator
- RELATED_PREVIOUS_REVIEWS:
  - `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW`
  - `AUDIT-20260325-CONTRACT-HARDENING-V1-SMOKETEST-CLOSEOUT-REVIEW`
- SCOPE:
  - `AUDIT_20260321_PARALLEL_WP1_V3_PRODUCT_SPEC_ALIGNMENT` as the historical failure baseline
  - `WP-1-Structured-Collaboration-Schema-Registry-v4` as the first orchestrator-managed recovery pass
  - `WP-1-Structured-Collaboration-Contract-Hardening-v1` as the first merge-contained contract-hardening closeout
  - `WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1` as the latest orchestrator-managed follow-on run
  - Integrated product code on local `main` through `154445243c0b28dc910454b97b0f7df2935529c7`
  - Operator-visible orchestration behavior observed during the live sessions on 2026-03-24 and 2026-03-25
- RESULT:
  - PRODUCT_REMEDIATION: PASS; the targeted structured-collaboration product gaps were eventually repaired on `main`
  - MASTER_SPEC_AUDIT: PARTIAL; the signed product scopes are closed, but adjacent structured-collaboration viewer debt still remains
  - WORKFLOW_DISCIPLINE: FAIL; the orchestrator-managed lane did not stay inside its own declared procedure
  - ACP_RUNTIME_DISCIPLINE: FAIL; runtime/session-control truth still required repair and status synchronization after technical work was already done
  - MERGE_PROGRESSION: PARTIAL; the sequence now demonstrates successful merge-contained closeout, but the first run falsely implied completion before `main` containment
- KEY_COMMITS_REVIEWED:
  - `d05dc08` initial Schema Registry v4 remediation handoff
  - `511dc5e` Schema Registry v4 queue-reason repair
  - `c6e8ba2` Contract Hardening v1 merged closeout on `main`
  - `154445243c0b28dc910454b97b0f7df2935529c7` Governed Next Action Alignment v1 merged closeout on `main`
- EVIDENCE_SOURCES:
  - `.GOV/Audits/audits/AUDIT_20260321_PARALLEL_WP1_V3_PRODUCT_SPEC_ALIGNMENT.md`
  - `.GOV/Audits/smoketest/AUDIT_20260325_SCHEMA_REGISTRY_V4_SMOKETEST_RECOVERY_REVIEW.md`
  - `.GOV/Audits/smoketest/AUDIT_20260325_CONTRACT_HARDENING_V1_SMOKETEST_CLOSEOUT_REVIEW.md`
  - `.GOV/task_packets/WP-1-Structured-Collaboration-Schema-Registry-v4/packet.md`
  - `.GOV/task_packets/WP-1-Structured-Collaboration-Contract-Hardening-v1/packet.md`
  - `.GOV/task_packets/WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1/packet.md`
  - `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Schema-Registry-v4/THREAD.md`
  - `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Contract-Hardening-v1/THREAD.md`
  - `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1/THREAD.md`
  - `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1/RUNTIME_STATUS.json`
  - `../gov_runtime/roles_shared/SESSION_CONTROL_REQUESTS.jsonl`
  - `../gov_runtime/roles_shared/SESSION_CONTROL_RESULTS.jsonl`
  - `../gov_runtime/roles_shared/SESSION_CONTROL_OUTPUTS/**/*.jsonl`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
  - Live operator-visible chat observations on 2026-03-24 and 2026-03-25
- RELATED_GOVERNANCE_ITEMS:
  - `RGF-01`
  - `RGF-02`
  - `RGF-03`
  - `RGF-04`
  - `RGF-05`
  - `RGF-06`
  - `RGF-07`
- RELATED_CHANGESETS:
  - `GOV-CHANGE-20260325-01`
  - `GOV-CHANGE-20260325-02`
  - `GOV-CHANGE-20260325-03`
  - `GOV-CHANGE-20260325-04`
  - `GOV-CHANGE-20260325-05`
  - `GOV-CHANGE-20260325-06`

---

## 1. Executive Summary

The orchestrator-managed WP workflow can now produce real product repairs on `main`, but it still cannot honestly be called a clean governed procedure. The product side improved materially across the sequence. The workflow side did not improve at the same rate.

What actually succeeded:

- the v3 false PASS was corrected by real follow-on product work
- the direct coder <-> validator communication lane became useful rather than ceremonial
- the later WPs reached actual `main` containment
- the latest run produced a deterministic replayable closeout on canonical `main`

What still failed:

- the operator had to correct core orchestrator-managed rules live
- the refinement-display rule was breached before being hardened
- helper-agent and auxiliary-topology boundaries were violated before being hardened
- the first validated PASS did not include merge progression to `main`
- later runs still needed status, parser, and session-control repair after the product work was already technically correct
- the latest run still slipped back into manual-WP habits by trying to ask the operator for a skeleton approval inside an orchestrator-managed lane

The correct conclusion is:

- product remediation is now real
- workflow discipline is still not trustworthy enough to be treated as solved
- operator burden remains too high for a workflow that claims autonomous orchestrator-managed execution

## 2. Lineage and What This Run Needed To Prove

After the governance hardening pass and the v3 false-PASS audit, the repo needed one thing more than anything else: proof that a single orchestrator-managed WP could run end to end without the operator repeatedly repairing the procedure.

That proof-of-workflow standard was stricter than "the code eventually got fixed." The lane needed to show:

- refinement is visible in chat before signature
- signature leads into packeted work without re-negotiating the workflow mid-run
- the Orchestrator uses ACP/CLI session steering and `just` commands rather than ad hoc role simulation
- product code is written only by the governed coder lane
- the declared role topology is the only live execution topology
- the operator is not asked to babysit skeleton checkpoints or workflow micro-steps
- merge authority closes on canonical `main`
- packet/runtime/task-board truth settle without repair-heavy backfill

### What Improved vs Previous Smoketest

- product correctness improved dramatically across the sequence:
  - Schema Registry v4 repaired the original shallow validator gap
  - Contract Hardening v1 closed the remaining governed-action, Task Board fidelity, and mailbox redaction validator gaps
  - Governed Next Action Alignment v1 closed the remaining compact-summary `next_action` drift on `main`
- the direct review lane improved:
  - the WP validator and integration validator both found real defects instead of rubber-stamping
  - review receipts and correlation chains became materially useful
- merge containment improved:
  - unlike Schema Registry v4, the latter two WPs reached `main` inside the governed run
- what did not improve enough:
  - operator burden stayed too high
  - status truth still lagged actual technical truth
  - ACP/session-control still needed repair instead of settling cleanly
  - the Orchestrator still slipped back into manual-step behavior that is not valid for an orchestrator-managed lane

## 3. Product Outcome

This review is not claiming that the product work failed. The product work was the strongest part of the sequence.

The sequence did eventually repair the concrete structured-collaboration gaps that motivated the smoke reviews:

- shared validator hardening
- canonical queue-reason vocabulary enforcement
- registry-backed `allowed_action_ids`
- Task Board authoritative-field drift checks
- mailbox redaction validation hardening
- governed compact-summary `next_action` alignment

That product work now exists on local `main` through:

- `c6e8ba2bf23ff9061b20f83a31567a6e47b322fe`
- `154445243c0b28dc910454b97b0f7df2935529c7`

The product-side adjacent debt is narrower now:

- the latest validator report still records that Task Board row records do not yet expose inline `next_action`; viewers currently rely on `summary_ref`
- that is real adjacent spec debt, but it does not erase the fact that the signed WP scopes themselves were materially closed

Net product judgment:

- the product lane is now substantially healthier than the workflow lane that produced it

## 4. Timeline

- 2026-03-21:
  - `AUDIT_20260321_PARALLEL_WP1_V3_PRODUCT_SPEC_ALIGNMENT` proved the v3 PASS claim was incomplete.

- 2026-03-24 to 2026-03-25:
  - `WP-1-Structured-Collaboration-Schema-Registry-v4` ran as the first orchestrator-managed recovery pass.
  - product scope was corrected, but workflow discipline failed on refinement display, helper-agent boundary, ACP repair, and merge progression.

- 2026-03-25:
  - `WP-1-Structured-Collaboration-Contract-Hardening-v1` repaired the follow-on product gaps and reached `main`.
  - workflow discipline improved, but session-control truth and validator closeout parsing still needed repair after merge.

- 2026-03-25:
  - `WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1` closed the remaining compact-summary `next_action` gap on `main`.
  - direct review remained strong, but the run still showed manual-step relapse, scope drift against live `main`, packet-status lag, and closeout synchronization lag.

## 5. Failure Inventory

### 5.1 Critical: the operator had to restate core orchestrator-managed law during active runs

Evidence:

- live operator-visible corrections required statements equivalent to:
  - do not use helper agents for product code
  - orchestrator-managed WPs should not keep interacting with the operator mid-run
  - use ACP and full models in terminals, not ad hoc sub-agents

Reason:

- the Orchestrator mixed old manual-WP habits with the newer orchestrator-managed ACP model.

Impact:

- the operator became a workflow repair surface instead of a high-level approver
- the lane could not honestly claim autonomous orchestrator-managed execution

Judgment:

- this is the strongest workflow-discipline failure in the sequence

### 5.2 Critical: the refinement-display rule was breached before signature hardening landed

Evidence:

- repeated operator-visible corrections stated that the refinement had not actually been shown in chat
- later governance changes had to explicitly say shell output does not count as chat-visible refinement proof

Reason:

- the workflow treated tool output as if it were user-visible assistant-authored content

Impact:

- a one-time signature was nearly consumed without the required human-visible refinement review

Judgment:

- this is a genuine protocol breach, not a cosmetic UX issue

### 5.3 Critical: product-code delegation boundaries were violated before policy hardening

Evidence:

- the operator had to explicitly forbid product-code use of helper agents and require ACP/full-model coder lanes instead
- governance policy surfaces were later hardened to encode that boundary

Reason:

- the boundary between governance assistance and product coding was not enforced hard enough before action

Impact:

- the orchestrator-managed model temporarily drifted away from its declared execution method

Judgment:

- this is a workflow-authority failure by the Orchestrator

### 5.4 High: Schema Registry v4 reached validated PASS without canonical `main` containment

Evidence:

- the earlier smoketest review recorded that `511dc5e` was not contained in `main` or `origin/main` after the WP had been described as complete

Reason:

- workflow closeout and technical validation were allowed to drift apart

Impact:

- the repo carried split truth between "validated" and "actually integrated"

Judgment:

- this is the clearest demonstration that merge progression still needs to be a hard gate

### 5.5 High: undeclared auxiliary worktrees existed outside the packet-declared topology

Evidence:

- the sequence created or retained undeclared auxiliary checkouts such as:
  - `wtc-schema-registry-v4-check-511dc5e`
  - `wtc-schema-registry-v4-postwork`
  - `handshake-wp1-schema-validator-511dc5e`

Reason:

- the workflow tolerated convenience worktrees outside the declared packet topology

Impact:

- worktree truth became harder to audit
- the workflow could no longer honestly claim that packet-declared role surfaces were the only execution surfaces

Judgment:

- this is a governance-procedure failure, not harmless cleanup residue

### 5.6 High: the Orchestrator relapsed into manual checkpoint behavior inside an orchestrator-managed lane

Evidence:

- during the latest WP, the operator had to restate that this workflow should not stop for mid-run approvals and should instead steer ACP sessions through `just` commands until completion or a real blocker
- the run had attempted to present a skeleton-approval checkpoint to the operator

Reason:

- the Orchestrator reused a manual approval pattern that is not valid for the signed lane

Impact:

- the operator was pulled back into the control loop unnecessarily

Judgment:

- this is not a small phrasing error; it changes who is actually driving the run

### 5.7 High: integration authority had to operate through live scope drift against current `main`

Evidence:

- the latest WP thread records:
  - integration-validator FAIL on `d8ac5fc` because current `main` no longer matched the feature-branch assumptions around `profile_extension`
  - coder follow-up widening the candidate to `0da6ea3` by touching `src/backend/handshake_core/src/storage/locus_sqlite.rs`
  - integration-validator FAIL because the packet still limited signed scope to three files
  - final integration succeeded only by carrying the exact signed-scope file state onto `main`

Reason:

- signed packet scope, live branch compatibility, and current `main` drift were not preflighted tightly enough

Impact:

- merge authority had to do extra reasoning to preserve signed scope without silently broadening the packet

Judgment:

- the integration-validator did the right thing
- the workflow around it was still too fragile

### 5.8 High: ACP/session-control truth still required repair instead of settling mechanically

Evidence:

- Contract Hardening v1 required later settlement of request `f13de206-76ee-4e8b-aa73-a8218187d9df`
- the earlier Schema Registry v4 run also showed concurrent-run rejection and broker repair behavior

Reason:

- request/output/result ledgers still allow states where humans can infer the outcome, but the canonical results ledger remains incomplete

Impact:

- runtime truth can appear healthy only after repair

Judgment:

- this is a control-plane integrity failure, not just logging debt

### 5.9 Medium: packet, board, and runtime truth still lagged actual technical truth

Evidence:

- the latest packet validation report explicitly notes that packet status lagged as `Ready for Dev` during merge-authority review and was corrected during closeout
- the packet also notes that packet/task-board/runtime truth lagged until final synchronization

Reason:

- status synchronization still occurs after technical merge truth rather than atomically with it

Impact:

- observers can read contradictory truths during a supposedly completed run

Judgment:

- this is milder than missing merge containment, but it is still workflow dishonesty if normalized

### 5.10 Medium: shared worktree noise remained high enough to complicate proof surfaces

Evidence:

- validator thread entries repeatedly refer to unrelated dirty product files outside packet scope
- earlier reviews also recorded `.GOV` shared-junction noise and post-work sensitivity to unrelated drift

Reason:

- WP worktrees are still not hygienic enough by default for deterministic closeout

Impact:

- validators and integration authority must spend energy separating real packet evidence from environmental dirt

Judgment:

- this is a systemic workflow-quality problem, even when the final technical judgment is still correct

### 5.11 Medium: governance state modeling is still incomplete

Evidence:

- earlier steering confused "failed historical closure" with "live smoke-test baseline"
- the repo still needed operator clarification to treat the v3 pair as active smoke-test recovery targets rather than dead legacy artifacts

Reason:

- governance state vocabulary is still not expressive enough

Impact:

- the system can aim at the wrong operational objective until the operator corrects it

Judgment:

- this is a modeling problem, not a single-run accident

## 6. Role Review

### 6.1 Orchestrator Review

Strengths:

- chose the correct follow-on product scopes after each previous review
- kept the later product WPs narrow and spec-anchored
- eventually drove both later WPs through actual `main` containment
- converted several earlier failures into explicit governance policy hardening

Failures:

- violated refinement-display procedure before hardening it
- violated product-code helper-agent boundary before hardening it
- allowed validated PASS without merge containment in Schema Registry v4
- tolerated undeclared auxiliary worktrees
- relapsed into manual checkpoint behavior in a lane that should have remained autonomous
- still relied on post-technical-truth repair to settle governance truth

Assessment:

- strongest role in recovery momentum, but also the role most responsible for procedure drift
- technically effective, procedurally unreliable

### 6.2 Coder Review

Strengths:

- produced real product repairs, not shallow narrative compliance
- responded to validator findings in bounded revision cycles
- later WPs stayed close to the intended product surface

Failures:

- initial handoffs in multiple runs still missed one real defect each
- branch-head compatibility with current `main` was not fully anticipated in the latest run
- worktree hygiene remained noisy enough that validators kept needing narrow-commit discipline

Assessment:

- good technical lane overall
- still needs stronger self-audit for compatibility and emitted-contract boundaries before first handoff

### 6.3 WP Validator Review

Strengths:

- this role became the most reliable source of real negative proof
- caught concrete defects in the product lane instead of narrating generic caution
- guarded signed scope when the latest branch-head widened beyond packet scope

Failures:

- no material current-run validator failure comparable to the earlier false-PASS history was observed in these later runs

Assessment:

- strongest procedural role in the later sequence
- review credibility is materially recovered, even if older historical trust debt still exists

### 6.4 Integration Validator Review

Strengths:

- blocked invalid merge progression when scope or compatibility was not defensible
- later runs actually landed the approved scope on `main`
- latest run preserved signed-scope discipline by integrating only the exact approved file state

Failures:

- still operated inside a workflow where packet status and runtime truth lagged behind technical merge truth

Assessment:

- strong technical authority behavior
- the surrounding workflow system still makes the role work harder than it should

## 7. Review Of Coder and Validator Communication

This is the strongest part of the orchestrator-managed model today.

What worked:

- structured direct-review receipts were actually used
- validator feedback was concrete, spec-anchored, and diff-aware
- coder responses were revision-oriented rather than defensive
- the integration-validator lane used real technical and scope objections, not ceremonial approval language

What still concerns me:

- good communication did not prevent runtime-truth lag
- good communication did not prevent status or topology drift
- the communication contract is ahead of the control plane around it

Net judgment:

- preserve this lane as the baseline
- do not mistake good review traffic for a healthy overall workflow

## 8. ACP Runtime / Session Control Findings

The workflow still proves that ACP/runtime truth is weaker than the product and review lanes that depend on it.

Observed across the sequence:

- session-control requests can still require later settlement
- concurrent-run rejection and repair logic still occur during normal final-lane operation
- packet status, runtime status, and technical merge truth still do not settle atomically
- the final green state is often a repaired state, not the first state produced by the governed helpers

The latest runtime status for the governed-next-action WP is clean at rest:

- `current_packet_status: "Validated (PASS)"`
- `runtime_status: "completed"`

That clean final state does not erase the fact that the route to it still involved lag and repair.

## 9. Governance Implications

This review says the governance kernel is directionally correct but not yet operationally strict enough.

The biggest implications are:

- the workflow needs explicit invalidity rules, not just best-practice prose
- operator intervention on core protocol should be treated as workflow failure evidence, not a normal part of the lane
- autonomous orchestrator-managed WPs need a stronger distinction from manual packet flows
- declared topology, merge containment, and status truth must all become hard gates

The repo is now at the point where product quality is no longer the main uncertainty. Workflow legitimacy is.

## 10. Positive Signals Worth Preserving

- product remediation is real and now substantially deeper than the original false-PASS baseline
- direct coder <-> validator receipts are working
- validators are providing real negative proof
- merge authority on canonical `main` now exists as a demonstrated success path
- the latest WP can be replayed deterministically on canonical `main`
- governance improvements made after earlier failures were meaningful, even if not yet sufficient

## 11. Remaining Product or Spec Debt

This review is about workflow, but it should not erase the last visible adjacent product debt:

- Task Board row records still expose `summary_ref` instead of an inline `next_action` field, which remains a narrower adjacent viewer-level spec delta

That product debt should stay visible, but it is not the main blocker to trusting the workflow. Workflow legitimacy is the blocker now.

## 12. Post-Smoketest Improvement Rubric

### 12.1 Workflow Smoothness

- TREND: IMPROVED
- CURRENT_STATE: HIGH
- Evidence:
  - later runs reached real `main` containment, but the operator still had to restate core lane rules, runtime/session truth still needed repair, and the latest run still relapsed into manual checkpoint behavior
- What improved:
  - merge-contained closeout now exists as a demonstrated success path
  - direct review traffic is much cleaner than the earlier false-PASS history
- What still hurts:
  - too much Orchestrator repair work
  - too much operator protocol correction
  - non-atomic closeout and lagging status truth
- Next structural fix:
  - make operator restatement of core orchestrator-managed rules an automatic workflow-invalidity signal and block normal progression until the lane is reset

### 12.2 Master Spec Gap Reduction

- TREND: IMPROVED
- CURRENT_STATE: LOW
- Evidence:
  - the structured-collaboration gaps that motivated Schema Registry v4, Contract Hardening v1, and Governed Next Action Alignment v1 were materially reduced from broad validator and contract debt to one narrower adjacent viewer-level debt item
- What improved:
  - the remaining gap list is now much smaller and more explicit than the v3/v4 baseline
  - validators produced real negative proof instead of shallow PASS narration
- What still hurts:
  - one adjacent Task Board viewer/row exposure gap remains visible
- Next structural fix:
  - keep the next product follow-on narrow and viewer-contract-specific rather than reopening the broader structured-collaboration family

### 12.3 Token Cost Pressure

- TREND: IMPROVED
- CURRENT_STATE: HIGH
- Evidence:
  - narrower packets, better review receipts, and actual merge-contained closeout reduced waste, but repeated operator clarifications, repeated Orchestrator steering, runtime repair, and status-sync lag still consumed unnecessary effort
- What improved:
  - the review loop itself is cheaper because validators now give concrete defects
  - product follow-ons are much narrower than the earlier recovery pass
- What still hurts:
  - workflow confusion still generates expensive steering and repair work
  - final truth still settles too late
- Next structural fix:
  - make final closeout atomic across merge truth, packet truth, runtime truth, and session-control settlement so future runs stop paying for after-the-fact repair

## 13. Silent Failures, Command Surface Misuse, and Ambiguity Scan

### 13.1 Silent Failures / False Greens

- Schema Registry v4 demonstrated the clearest false green:
  - the WP was described as complete while the approved fix was not yet on `main`
- later runs showed a milder version of the same pattern:
  - packet/runtime/task-board truth could lag actual technical truth after merge
- the refinement-display breach was also a silent failure:
  - the workflow behaved as if a mandatory human-visible phase had occurred when it had not actually occurred in chat

Judgment:

- these are not ordinary "process issues"
- they are false-green conditions and should be treated as invalid workflow states

### 13.2 Systematic Wrong Tool or Command Calls

- helper-agent product-code delegation was an invalid execution surface for this lane
- the latest run's mid-run skeleton-approval ask was the wrong workflow pattern for `WORKFLOW_LANE: ORCHESTRATOR_MANAGED`
- auxiliary worktrees such as detached check checkouts and post-work checkouts acted like off-protocol convenience tools rather than declared packet topology
- repeated late repair steps also suggest command-surface gaps:
  - the lane still lacked one clean "close everything truthfully" path, so humans and helpers kept stitching together multiple commands

Judgment:

- these were not random slips
- they are signs that the command surface still leaves too much room for choosing the wrong tool family for the lane

### 13.3 Task and Path Ambiguity

- the repo still allowed confusion between:
  - failed historical closure
  - active live smoketest baseline
- the latest run still had signed-scope versus current-`main` compatibility ambiguity
- undeclared auxiliary worktrees created path-truth ambiguity about which checkout was authoritative
- packet status lag created truth ambiguity about whether the run was still `Ready for Dev` or already merge-authority validated

Judgment:

- the path and source-of-truth story is better than before, but still not crisp enough to prevent expensive late clarification

### 13.4 Read Amplification / Governance Document Churn

- the operator-visible pattern of repeated governance-document inspection and repeated command-surface checking is itself evidence of ambiguity
- that churn should be expected from the Orchestrator only in a small amount during setup, not repeatedly during active execution
- the same smell should be treated as a problem when it appears in coder or validator lanes:
  - repeated rereading of protocols
  - repeated command rediscovery
  - repeated path/worktree/source-of-truth revalidation after startup without a real context change

Why this matters:

- it burns tokens
- it slows role execution
- it usually means the active packet, startup prompt, or command surface is not sharp enough

Judgment:

- repeated governance-document churn is not "carefulness"
- it is review evidence that the workflow still contains ambiguity

### 13.5 Hardening Direction

- make repeated rereading and command rediscovery an explicit review signal instead of invisible token waste
- keep shrinking the live read set each role needs after startup
- prefer one canonical command or helper per lifecycle step instead of multiple near-equivalent surfaces
- keep moving ambiguity from live execution into startup prompts, packet fields, helper commands, and hard gates

## 14. Suggested Remediations

### Governance / Runtime

- make "operator had to restate core lane rules during active run" an explicit workflow failure signal
- add a hard rule for `WORKFLOW_LANE: ORCHESTRATOR_MANAGED`:
  - after signature, the Orchestrator must not ask the operator for routine mid-run approvals, skeleton checkpoints, or handholding unless a real blocker or policy conflict exists
- reject undeclared auxiliary worktrees for active WPs
- make merge containment a hard completion gate
- make closeout atomic across:
  - merge truth
  - packet status
  - task-board status
  - runtime status
  - session-control settlement
- require exactly one settled terminal result row for every control request
- add a preflight that compares signed packet scope against current `main` compatibility before final authority starts
- add a governed packet-widening mechanism for cases where integration reveals one unavoidable adjacent shared-surface file

### Product / Validation Quality

- keep the direct coder <-> validator lane mandatory
- keep negative proof mandatory
- require compatibility probes against current `main` for narrow packets touching shared schemas or runtime emitters

### Documentation / Review Practice

- preserve the smoketest template, but extend it for workflow audits with:
  - Protocol Deviations
  - Operator Burden
  - Authority Surface Drift
  - Invalidity Rules For Future Runs
- keep distinguishing `Handshake (Product)` from `Repo Governance` in operator-facing reasoning
- preserve stable audit IDs so governance follow-on work can cite this review directly

## 15. Command Log

- `git status --short` -> PASS
- `Get-Content .GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md` -> PASS
- `Get-Content .GOV/Audits/smoketest/AUDIT_20260325_SCHEMA_REGISTRY_V4_SMOKETEST_RECOVERY_REVIEW.md` -> PASS
- `Get-Content .GOV/Audits/smoketest/AUDIT_20260325_CONTRACT_HARDENING_V1_SMOKETEST_CLOSEOUT_REVIEW.md` -> PASS
- `Get-Content .GOV/task_packets/WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1/packet.md` -> PASS
- `Get-Content ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1/THREAD.md` -> PASS
- `Get-Content ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1/RUNTIME_STATUS.json` -> PASS
- `rg -n "WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1|f13de206-76ee-4e8b-aa73-a8218187d9df|Concurrent governed run already active" ../gov_runtime/roles_shared/SESSION_CONTROL_REQUESTS.jsonl ../gov_runtime/roles_shared/SESSION_CONTROL_RESULTS.jsonl ../gov_runtime/roles_shared/SESSION_CONTROL_OUTPUTS -g "*.jsonl"` -> PASS

## 16. Protocol Deviations and Invalid Procedure Calls

This section is intentionally outside the base template because the base template does not currently make workflow invalidity explicit enough.

The following behaviors should be considered protocol deviations, not "workflow noise":

- requesting signature before the refinement is visibly in chat
- using helper agents for product-code writing without explicit operator approval and packet evidence
- asking the operator for routine skeleton or mid-run approvals inside an orchestrator-managed lane
- treating validated PASS as equivalent to merge completion
- allowing undeclared auxiliary worktrees to exist as active WP execution surfaces
- allowing session-control outputs without settled terminal result truth
- letting packet status lag behind actual technical authority truth during final authority

## 17. Operator Burden and Unexpected Manual Interventions

The operator had to intervene in ways that should not have been necessary for a clean orchestrator-managed lane:

- restating that the refinement must actually appear in chat
- restating that product code must be handled through ACP/full-model lanes rather than helper agents
- restating that orchestrator-managed WPs should not keep interacting with the operator until completion or real blocker
- explicitly approving cleanup of undeclared worktrees that should not have existed in the first place

This matters because the workflow is supposed to reduce operator supervision, not convert the operator into a live protocol corrector.

## 18. Authority Surface Drift Map

The sequence showed several different types of truth drift:

- Chat truth drift:
  - refinement requirement existed, but chat-visible refinement had not actually happened
- Role-topology drift:
  - helper-agent/product-code ambiguity and undeclared auxiliary worktrees
- Merge-truth drift:
  - validated PASS existed before `main` containment in Schema Registry v4
- Runtime-truth drift:
  - control outputs could imply final state before settled ledger truth existed
- Status-truth drift:
  - packet/task-board/runtime status lagged actual technical merge truth
- Scope-truth drift:
  - latest branch-head compatibility repair widened beyond signed packet scope, forcing merge authority to integrate only the exact approved file state

This drift map is the clearest reason the workflow cannot yet be called clean, even though the product results are now much better.

## 19. What Must Count As Invalid Next Time

The following should be treated as automatic workflow invalidity indicators in future orchestrator-managed smoke runs:

- operator has to restate a core lane rule during active execution
- signature requested before refinement is visibly in chat
- product code written outside the governed coder lane
- any undeclared WP worktree appears in active use
- packet says validated PASS while `main` does not contain the approved commit
- final authority starts without current `main` compatibility preflight
- final closeout leaves status or session-control truth waiting for later repair

If these are not treated as invalidity conditions, the workflow will keep reporting success on runs that only succeeded because the operator or the Orchestrator repaired the process in flight.
