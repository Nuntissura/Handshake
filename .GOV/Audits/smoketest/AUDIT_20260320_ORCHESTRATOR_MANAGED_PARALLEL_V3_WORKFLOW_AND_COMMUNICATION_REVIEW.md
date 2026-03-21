# Audit: Orchestrator-Managed Parallel v3 Workflow, Governance Patch, and Communication Review

## METADATA
- AUDIT_ID: AUDIT-20260320-ORCH-MANAGED-PARALLEL-V3-WORKFLOW-COMMUNICATION-REVIEW
- DATE_UTC: 2026-03-20
- AUTHOR: Codex acting as Orchestrator
- SCOPE:
  - `WP-1-Structured-Collaboration-Schema-Registry-v3`
  - `WP-1-Loom-Storage-Portability-v3`
- RESULT: COMPLETE WITH SUCCESSFUL DELIVERY, MATERIAL LIVE GOVERNANCE PATCHING, AND UNDERUSED CODER-VALIDATOR COMMUNICATION
- KEY_COMMITS_REVIEWED:
  - `92fc9c2` `gov: folderize v3 work packets + allow orchestrator main sync`
  - `f839460` `gov: close v3 WPs and repair runtime truth`
  - `b5efe84` `gov: restore startup compatibility surfaces`
  - `7f183d9` `gov: backfill legacy repo-local wp communications`
  - `e867469` `merge: selective Loom v3 integration from 7aa995b`
  - `fe998e1` `merge: selective Schema Registry v3 integration from 23f4c9a`
  - `dcb7f0c` `gov: sync governance kernel 7f183d9`
- EVIDENCE_SOURCES:
  - `.GOV/task_packets/WP-1-Loom-Storage-Portability-v3/packet.md`
  - `.GOV/task_packets/WP-1-Structured-Collaboration-Schema-Registry-v3/packet.md`
  - `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Loom-Storage-Portability-v3/THREAD.md`
  - `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Loom-Storage-Portability-v3/RECEIPTS.jsonl`
  - `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Schema-Registry-v3/THREAD.md`
  - `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Schema-Registry-v3/RECEIPTS.jsonl`
  - `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Loom-Storage-Portability-v3/RUNTIME_STATUS.json`
  - `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Schema-Registry-v3/RUNTIME_STATUS.json`
  - `../gov_runtime/roles_shared/ROLE_SESSION_REGISTRY.json`
  - `../gov_runtime/roles_shared/SESSION_CONTROL_RESULTS.jsonl`
  - `../gov_runtime/roles_shared/SESSION_CONTROL_OUTPUTS/*WP-1-*-v3/*.jsonl`

---

## 1. EXECUTIVE SUMMARY

Both v3 WPs were delivered and integrated successfully, but the workflow was not low-touch. The run required substantial live governance repair before it became trustworthy enough to execute end to end.

The main conclusion is not "the workflow failed." The main conclusion is:

- product delivery succeeded
- the new packet/refinement/micro-task structure materially helped
- validator rigor was better than the earlier false-PASS attempts
- the official coder-validator communication lane was barely used
- the Orchestrator therefore absorbed too much steering and truth-repair work

Your instinct about the steering load is correct. It was high partly because governance was being patched live, but also because the coder-validator loop never became the primary execution path on the official governed surfaces. The workflow still behaved more like "Orchestrator-centered coordination with strong packets" than true low-touch parallel collaboration.

---

## 2. FINDINGS BY SEVERITY

### 2.1 Critical: The initial restart state was not governance-safe

At restart time, the v3 packets existed, but the live governance truth was inconsistent across packet status, PREPARE records, runtime files, and actual worktree/branch existence.

Observed defects:

- v3 refinements had stale `SPEC_TARGET_SHA1`
- PREPARE records still pointed at deleted worktrees
- runtime files still claimed active working state against deleted worktrees
- packet/shared-record truth was not aligned with restart reality

Impact:

- clean governed restart was impossible
- startup could produce misleading signals about what was active
- the Orchestrator had to repair workflow truth before launch

Assessment:

- this is the single biggest governance failure in the run
- if left unfixed, it would have created another false "started" or false "ready" state

### 2.2 High: ACP plugin-first launch did not prove reliable enough for low-drama parallel execution

The session registry shows the v3 governed sessions needed CLI escalation after repeated plugin timeouts.

Observed evidence:

- each v3 coder/WP-validator session ended with `plugin_failure_count: 2`
- each ended with `plugin_last_result: PLUGIN_TIMED_OUT`
- each ended with `cli_escalation_used: true`

Impact:

- launch path was slower and noisier than intended
- Orchestrator supervision cost increased
- shell/plugin result could not be treated as a clean source of truth

Assessment:

- plugin-first remains a reasonable policy target
- it is not yet reliable enough to be treated as a low-friction default under pressure

### 2.3 High: Direct coder-validator communication did not happen on the official governed channel

This is the strongest explanation for the excessive steering load.

For both v3 WPs, the official communication receipts show only:

- `SYSTEM | ASSIGNMENT`
- `ORCHESTRATOR | HEARTBEAT`
- `ORCHESTRATOR | THREAD_MESSAGE`

There were no official `CODER` receipts and no official `WP_VALIDATOR` receipts in either WP communication ledger.

Impact:

- the governed communication artifacts proved liveness, not collaboration
- the Orchestrator became the practical review router and sequencing engine
- validator challenge happened mostly through packet/report discipline, not through direct back-and-forth

Assessment:

- this is the main workflow-design failure that remained even after the governance fixes
- without direct role-to-role exchange, parallel work still centralizes around the Orchestrator

### 2.4 High: Legacy/current runtime path migration remained incomplete and leaked into live workflow

Current governance had already moved toward externalized runtime authority, folderized packets, and canonical packet-path resolution. Older packet/runtime surfaces still existed and were capable of breaking current checks.

Observed consequences:

- `wp-communications-check` needed to be changed so only the packet-authoritative communication root is validated
- legacy repo-local communication folders had to be backfilled so older packets stopped breaking startup and `gov-check`

Impact:

- historical compatibility debt could block current work
- governance startup was more brittle than the product execution itself

Assessment:

- `b5efe84` and `7f183d9` were necessary
- `7f183d9` is a compatibility shim, not a final architectural win

### 2.5 Medium: The live `.GOV` junction model still creates scope noise inside coder worktrees

Coder session logs clearly saw broad `.GOV` churn while operating in product worktrees. This is expected under the live-junction architecture, but it is still a workflow hazard.

Impact:

- coders see unrelated governance drift in `git status`
- scope policing requires extra discipline and extra steering
- product lanes become visually noisy even when governance changes are legitimate and external

Assessment:

- the junction model is still the right architecture
- the human factors around it are not yet good enough

### 2.6 Medium: Validator gate state had a real persistence bug

`validator_gates.mjs` needed a repair so `saveWpState(...)` preserved `committed_validation_evidence` instead of dropping it during later gate writes.

Impact:

- a validator could do the correct committed-handoff check and still lose the recorded proof
- later gate progression could misread the WP as unproven

Assessment:

- this was a real correctness bug, not cosmetic cleanup
- fixing it materially reduced the chance of false gate blockage or false ambiguity

### 2.7 Medium: `sync-gov-to-main` was too rigid for real closeout conditions

`sync-gov-to-main.mjs` needed override support so governance could be synced into a clean integration clone instead of assuming one specific permanent clone was the only viable target.

Impact:

- mainline governance sync was too tightly coupled to one local topology assumption
- cleanup and final integration were harder than necessary

Assessment:

- this patch was justified
- the sync flow is now more operationally realistic

---

## 3. REVIEW OF THE GOVERNANCE PATCHES

### 3.1 `92fc9c2` was the largest and most structurally useful patch

What it did well:

- moved the two v3 WPs into folderized packet layout
- co-located `packet.md`, `refinement.md`, and generated `MT-*.md`
- upgraded the governance/runtime/session tooling to resolve folderized packets
- enabled orchestrator-authorized `sync-gov-to-main`

Why it mattered:

- the coder and validator both benefited from the packet folder format
- micro-tasks created a clearer execution contract than the earlier flat packets

Risk:

- this was a large governance sweep landed close to live execution
- it improved the system, but it also increased the chance of migration drift

Judgment:

- good patch
- too much surface area was changed immediately before a live run

### 3.2 `f839460` was a necessary truth-repair patch

What it did well:

- normalized packet closeout truth
- repaired runtime state for the v3 WPs
- preserved committed validation evidence
- synchronized records needed for honest closure

Why it mattered:

- without it, the run could have ended in another "work done but truth surfaces disagree" state

Risk:

- it mixes closure repair, runtime correction, and gate-bug repair
- that is operationally efficient, but it couples multiple concerns

Judgment:

- necessary and correct
- should eventually be decomposed into smaller independently testable governance changes

### 3.3 `b5efe84` was the correct narrow compatibility fix

What it did well:

- changed `wp-communications-check` so it validates only the authoritative communication root for a packet
- stopped stale non-authoritative folders from breaking current packets

Why it mattered:

- this is the correct direction under mixed-mode migration

Judgment:

- strong patch
- this is closer to a root-cause fix than a temporary workaround

### 3.4 `7f183d9` solved the immediate startup problem, but it is a debt-carrying shim

What it did well:

- backfilled missing repo-local communication artifacts for legacy packets
- restored startup/gov-check compatibility without touching product code

What concerns me:

- the patch preserves old topology assumptions instead of eliminating them
- if left in place indefinitely, it will blur which runtime roots are still authoritative

Judgment:

- acceptable as a short-term compatibility bridge
- should be retired once all legacy packet declarations are migrated or sunsetted

---

## 4. RISKS, ERRORS, BUGS, AND CONCERNS

### 4.1 Risks that remain

- plugin launch reliability is still not good enough to trust as the main success signal
- compatibility shims can accumulate and make authority harder to reason about
- coders still see governance churn in product lanes
- official WP communications are still too easy to bypass in practice

### 4.2 Errors and bugs found during the run

- stale PREPARE/runtime/spec truth for v3 restart
- validator gate evidence loss bug
- startup compatibility drift around mixed packet/runtime generations
- orphaned session-control request/result inconsistency that required ledger repair

### 4.3 Main concern

The product-side result is now stronger than the workflow-side leverage. That is backwards. A good governance system should reduce steering cost after the first repair wave. This run still required ongoing Orchestrator interpretation and intervention.

---

## 5. SYSTEMATIC IMPROVEMENTS FOR GOVERNANCE FAILURE MODES

### 5.1 Add a real "smoke-start hard gate"

Before any governed session launch, require one command that fails unless all of the following are true:

- `just gov-check`
- `just orchestrator-startup`
- every packet/refinement spec hash is current
- every PREPARE target exists on disk
- every packet/runtime/traceability record agrees on current phase and worktree
- communication artifacts exist at the packet-authoritative root only

This should have prevented the live restart repair work.

### 5.2 Make PREPARE and runtime truth a transactional write

`orchestrator-prepare-and-packet` should update these surfaces together or not at all:

- packet current state
- runtime status
- orchestrator gate registry
- build order / task board / traceability rows
- worktree path declarations

This would eliminate split-brain restart state.

### 5.3 Make direct coder-validator communication mandatory and machine-checked

For each MT or first active clause group:

- validator must publish one tripwire/checklist receipt to the coder
- coder must publish one response receipt stating intended proof/tests before editing
- validator must publish one post-handoff review receipt before verdict

If these receipts are absent, the WP should not be considered a healthy orchestrator-managed collaboration run.

### 5.4 Reduce Orchestrator steering after startup by design, not by hope

The Orchestrator should do:

- launch
- initial lane confirmation
- checkpoint approval
- gate repair only when needed

It should not have to be the normal path for scope clarification, review routing, or role wake-ups.

This requires stronger auto-triggers:

- coder MT selection -> auto-notify validator
- validator checklist posted -> auto-notify coder
- coder handoff appended -> auto-trigger validator review state

### 5.5 Treat plugin instability as a batch-level mode switch

If the bridge times out twice across the batch, do not keep rediscovering that one session at a time. Switch the run into explicit CLI mode for the rest of the batch and record that decision once.

### 5.6 Improve coder-lane ergonomics around live governance junctions

Keep the live junction architecture, but reduce noise:

- default coder status helpers should hide `.GOV` drift
- startup text should explicitly warn that `.GOV` changes may be unrelated live governance edits
- if possible, mount or present `.GOV` as read-only to coder sessions

### 5.7 Separate compatibility shims from final architecture

Every time a patch backfills legacy behavior, record:

- why it exists
- what newer surface supersedes it
- what condition allows deletion
- a target sunset date or migration trigger

This should be mandatory for patches like `7f183d9`.

### 5.8 Auto-generate a post-run audit skeleton

The system already has enough evidence to draft the first 70 percent of this audit automatically from:

- packet status
- runtime status
- session registry
- receipts/thread counts
- validator gates
- merge commits

That would make audits faster and more consistent.

---

## 6. REVIEW OF CODER <-> VALIDATOR COMMUNICATION

### 6.1 What worked

- The packet and micro-task structure gave coder and validator a common technical contract.
- Validator prep quality was materially better than in the earlier failed attempts.
- The validator sessions did real adversarial review work once handoff existed.
- Packet handoff/evidence sections were much stronger than the previous false-closeout cycles.

### 6.2 What failed

The official communication surface was effectively unused for actual coder-validator exchange.

For both v3 WPs:

- `THREAD.md` contains only the system initialization line plus one Orchestrator launch message
- `RECEIPTS.jsonl` contains only three entries: assignment, orchestrator heartbeat, orchestrator thread message
- there are zero official coder messages
- there are zero official validator messages

That means:

- the official governed channel did not capture technical back-and-forth
- communication happened mainly through packet edits, session-local work, and Orchestrator steering
- the workflow did not prove the direct collaboration model it is supposed to exercise

### 6.3 Why the steering load became high

Yes, live governance patching was part of it. It forced the Orchestrator to do runtime truth repair and path-compatibility work in the middle of delivery.

But that is not the whole explanation.

The deeper issue is that the coder-validator lane never became self-propelling on the official collaboration surface. When the governed channel is empty, the Orchestrator naturally becomes:

- the launcher
- the scope police
- the review sequencer
- the ambiguity resolver
- the truth synchronizer

So the steering load was high because of both:

- live governance repair
- weak direct role-to-role communication enforcement

I judge the second factor as the more important long-term problem.

### 6.4 Communication improvements for the next run

Require these as explicit governed artifacts:

1. Validator kickoff receipt
   - posted after MT selection
   - contains clause checklist and tripwires

2. Coder implementation-intent receipt
   - posted before substantial edits
   - states the exact proof plan and the tests intended to close the MT

3. Coder handoff receipt
   - posted after committed handoff state exists
   - references files, commands, results, and declared weak spots

4. Validator review receipt
   - posted before the governed report append
   - states PASS/FAIL direction, top risks, and any residual uncertainty

5. Gate enforcement
   - if the official thread/receipt surface lacks at least one direct two-way coder-validator exchange, the run should be marked communication-incomplete even if the code is correct

---

## 7. OVERALL JUDGMENT

This was a successful product run and an only partially successful workflow run.

What improved materially:

- packet/refinement quality
- micro-task clarity
- validator rigor
- closure honesty
- final governance/runtime truth

What did not improve enough:

- launch stability
- migration cleanliness
- direct coder-validator collaboration on the official surface
- Orchestrator workload reduction

Bottom line:

- the governance patches were mostly justified and mostly correct
- the run should be treated as proof that the system can finish real work under pressure
- it should not yet be treated as proof that the orchestrator-managed parallel workflow is low-friction or sufficiently decentralized

The next target should not be "more features." It should be "the same quality of delivery with much less Orchestrator steering."

---

## 8. POST-AUDIT REMEDIATION UPDATE - 2026-03-21

Since this audit, the first three communication remediations from Section 6.4 have been implemented on governance-only surfaces.

### 8.1 Landed changes

- Introduced a hard direct-review contract for new orchestrator-managed packets with `PACKET_FORMAT_VERSION >= 2026-03-21`.
- Added explicit packet fields `COMMUNICATION_CONTRACT: DIRECT_REVIEW_V1` and `COMMUNICATION_HEALTH_GATE: HANDOFF_VERDICT_BLOCKING`.
- Added new structured receipt kinds for the required coder <-> WP validator lane:
  - `VALIDATOR_KICKOFF`
  - `CODER_INTENT`
  - `CODER_HANDOFF`
  - `VALIDATOR_REVIEW`
- Added first-class helper commands for those receipts plus a machine gate:
  - `just wp-validator-kickoff`
  - `just wp-coder-intent`
  - `just wp-coder-handoff`
  - `just wp-validator-review`
  - `just wp-communication-health-check WP-{ID} {KICKOFF|HANDOFF|VERDICT}`
- Wired the communication health gate into real stop points:
  - coder `post-work`
  - validator `validator-handoff-check`
  - PASS commit clearance in `validator_gates`
- Updated governed startup prompts and role protocols so structured review helpers are now the default path and `wp-thread-append` is explicitly soft coordination only.

### 8.2 Effect on the audit findings

- This directly addresses Finding 2.3 by turning coder/WP-validator traffic into a machine-checked workflow requirement instead of a soft instruction.
- It reduces Orchestrator relay pressure by giving both sides one canonical structured exchange path plus blocking gates at handoff and verdict boundaries.
- It keeps migration risk bounded by applying the new contract only to new orchestrator-managed packets. Older packets remain readable and do not retro-fail because of the new law.
- It does not yet solve launch reliability, live `.GOV` junction noise, or richer monitor/test surfacing. Those remain open follow-up items.

### 8.3 Verification

- `node --check` passed for all changed governance scripts.
- `just gov-check` passed after the remediation landed.
- `just wp-communication-health-check WP-1-Structured-Collaboration-Schema-Registry-v3 STATUS` passed with the expected "contract not applicable" result because that packet predates the new contract (`2026-03-18`).
