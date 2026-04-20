# Validator Routing, Gates, and Closeout Repair

## Purpose

This document explains the validator side of the gov kernel as a control-plane subsystem.
It is not mainly about code review quality.
It is about how the kernel decides:

- which validator lane should act
- when a validator is allowed to act
- how PASS authority is constrained
- how report presentation is gated
- how closeout is mechanically repaired before final judgment

This area matters because a large fraction of governance cost is concentrated here:
handoff routing, gate enforcement, final-lane authority, and closeout convergence.

## Spec-Defined Semantics This Subsystem Serves

Before looking at the repo kernel mechanics, anchor the intended semantics in the current Master Spec.
The spec does not define `closeout-repair.mjs` or the validator gate files by name.
It defines the authority and completion model those scripts are trying to serve.

The strongest anchors for this topic are:

- canonical structured workflow records are the executable authority; Markdown mirrors are projections only
- Work Packets are authoritative execution contracts
- mailbox chronology and announce-back narrative are not authority by themselves
- when handoff or announce-back changes linked work meaning, explicit transcription into the authoritative artifact is required
- advisory announce-back must not imply completion; provenance gaps and missing transcription must block done-badge optimism

So in this document, `closeout` should be read as the current repo-kernel implementation of final-lane authority, authoritative transcription, and completion-safety duties.
It should not be read as if the current script topology fully defines the semantic scope of closeout in Handshake.

## Relationship to Other Research Documents

| Document | Role |
|---|---|
| `Gov_Kernel_Technical_Map.md` | Whole-system map |
| `Workflow_State_Packet_Truth_and_Range_Drift.md` | Truth-surface and range-drift deep dive |
| `ACP_Broker_and_Session_Control.md` | Launch and session-control deep dive |
| `Validator_Routing_Gates_and_Closeout_Repair.md` (this file) | Validator-side orchestration, gate enforcement, and closeout mechanics |
| `Harness_Lessons_Learned.md` | Cross-cutting lessons and implications |

## Main Command Surface

The validator subsystem is driven through a small command surface:

- `just validator-startup <WP_VALIDATOR|INTEGRATION_VALIDATOR|VALIDATOR>`
- `just validator-next <ROLE> [WP-ID]`
- `just integration-validator-context-brief WP-{ID}`
- `just wp-communication-health-check WP-{ID} [STAGE] [ROLE] [SESSION]`
- `just phase-check <PHASE> WP-{ID} ...`
- `just closeout-repair WP-{ID}`

Important point:

The command surface is thin, but the underlying logic is not.
The actual routing and authority decisions are made in validator-side libraries and checks rather than in the `just` wrappers themselves.

## Primary Code Surfaces

### Protocol and role boundary

- `.GOV/roles/wp_validator/WP_VALIDATOR_PROTOCOL.md`
- `.GOV/roles/integration_validator/INTEGRATION_VALIDATOR_PROTOCOL.md`
- `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`

These define the authority split:

- WP Validator owns per-microtask technical review
- Integration Validator owns whole-WP judgment and final automated verdict authority
- Orchestrator owns mechanical governance preparation and role launching

### Validator runtime entrypoint

- `.GOV/roles/validator/scripts/validator-next.mjs`

This is the validator resume/start-of-work entrypoint for all validator roles.
It is responsible for figuring out what the validator should look at next.

### Validator governance and routing logic

- `.GOV/roles/validator/scripts/lib/validator-governance-lib.mjs`

Key exported functions include:

- `loadValidatorCommunicationState`
- `deriveValidatorResumeState`
- `resolveValidatorActorContext`
- `evaluateValidatorPassAuthority`
- `buildValidatorReadyCommands`
- `evaluateValidatorPacketGovernanceState`
- `buildValidatorPacketCompleteResult`
- `buildValidatorHandoffCheckResult`

This file is one of the main brains of the validator subsystem.

### Gate enforcement and report-presentation pause

- `.GOV/roles/validator/checks/validator_gates.mjs`

This enforces the validator gate sequence around append, commit clearance, report presentation, and acknowledgment.

### Final-lane closeout logic

- `.GOV/roles/validator/scripts/lib/integration-validator-closeout-lib.mjs`
- `.GOV/roles/orchestrator/scripts/closeout-repair.mjs`

These files sit at the seam between final validator judgment and orchestrator-side mechanical truth repair.

### Supporting checks

- `.GOV/roles_shared/checks/phase-check.mjs`
- `.GOV/roles_shared/checks/wp-communication-health-check.mjs`
- `.GOV/roles/validator/checks/validator_gates.mjs`
- `.GOV/roles/validator/checks/validator-phase-gate.mjs`

## Role Split and Authority Model

### WP Validator

The WP Validator is not the final judge.
Its job is bounded:

- review MT-level implementation and handoff
- respond to coder review requests
- stay inside the coder branch/worktree review lane
- not mutate the coder worktree directly
- escalate after repeated failed fix cycles

### Integration Validator

The Integration Validator is the final automated verdict authority for orchestrator-managed WPs.
Its role is different:

- fresh-context whole-WP judgment
- no direct communication with the coder
- no individual MT review
- final merge-ready PASS/FAIL authority
- whole-WP closeout synchronization on the final lane

### Why this split matters

This is one of the stronger design decisions in the kernel.
It separates:

- local review / per-MT correction
- final authority / whole-WP judgment

The cost appears when routing, communication truth, or final-lane identity drift make the system uncertain about which validator lane is actually allowed to act.

## Validator Routing and Resume Model

`validator-next.mjs` does more than just "continue the validator."
It resolves what the validator should work on by combining:

- explicit CLI WP input
- inferred WP from prepare logs
- pending validator gate sessions
- validation-ready packets
- task-board state
- packet content
- communication state and runtime projections

That means validator routing is already a form of workflow-state interpretation, not just role continuation.

This is powerful, but it also means validator work can stall or misroute when:

- packet status is stale
- communication state is stale
- gate state is stale
- task-board state is stale

## Communication and Readiness Model

Validator readiness is not derived from packet status alone.
`validator-governance-lib.mjs` also reads:

- governed receipts
- thread history
- runtime status
- open review items
- next expected actor

The supporting check `wp-communication-health-check.mjs` evaluates:

- communication contract
- stage-specific route consistency
- pending notifications
- direct review boundary consistency

This is a strong idea.
The weakness is that communication truth is one more surface that can drift from packet truth and runtime truth.

## Pass Authority and Lane Identity

One of the most important checks in the validator subsystem is `evaluateValidatorPassAuthority`.
The kernel is trying to ensure that PASS authority is attributable to:

- the correct role
- the correct validator lane
- the correct session identity

This is reinforced by:

- actor-context resolution
- lane-specific session identity checks
- gate-session ownership checks
- final-lane governance invalidity detection in the closeout library

This is the right direction for a serious harness.
It prevents "some validator somewhere said pass" from being enough.

## Validator Gate Model

`validator_gates.mjs` implements a mechanical sequence rather than a single PASS flag.

The current gate actions are:

- `append {WP_ID} {PASS|FAIL|ABANDONED}`
- `commit {WP_ID}`
- `present-report {WP_ID} [PASS|FAIL|ABANDONED]`
- `acknowledge {WP_ID}`
- `status {WP_ID}`
- `reset {WP_ID}`

The intended meaning is:

1. The report is appended to the WP packet.
2. PASS is cleared for commit only after stronger checks pass.
3. The full validation report is presented in chat at the right moment.
4. User acknowledgment unlocks the next action.

Important enforcement details:

- minimum gate interval to prevent automation momentum
- wrong-lane/write-surface guard for governed validator gate writes
- legacy remediation guard to stop reuse of closed packets
- PASS commit clearance requires stronger proof than just "validator thinks it passes"

## What PASS Commit Clearance Actually Depends On

For PASS, gate clearance is not just a verdict.
The gate code checks for:

- durable committed validation evidence
- committed handoff validation against the PREPARE worktree source of truth
- verdict-ready direct review communication evidence
- integration-validator closeout preflight
- computed policy gate success

This is a major reason the validator subsystem feels heavy:
PASS is an aggregation of multiple workflow truths, not a single review outcome.

That is defensible for correctness.
It is expensive when those truths are stored across too many different surfaces.

## Closeout Repair as a Control-Plane Stage

`closeout-repair.mjs` is explicit about why it exists:

- it runs before Integration Validator launch
- it attempts to eliminate multi-retry closeout loops
- it exists because closeout failures were burning large amounts of token budget

The script currently classifies failures such as:

- baseline SHA mismatch
- missing signed-scope patch
- clause coverage mismatch
- missing validation verdict
- communication health issues
- integration-validator closeout-check failure

Then it tries known mechanical fixes and re-verifies.

This is a practical tool.
It is also direct evidence that closeout had become too repair-heavy to leave as a normal interactive loop.

### Closeout-Repair Failure-to-Fix Matrix

| Failure class | Typical detector | Primary truth surface in dispute | Current repair path | Mechanical or judgment-bound | Why it is expensive |
| --- | --- | --- | --- | --- | --- |
| `BASELINE_SHA_MISMATCH` | `closeout-repair.mjs` parsing `phase-check CLOSEOUT` output | packet field `CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA` versus current `handshake_main` HEAD | rewrite the packet field to the actual current main SHA, then re-run closeout phase-check | mostly mechanical | cheap to fix, but it proves packet truth can go stale independently of repo truth |
| `MISSING_SIGNED_SCOPE_PATCH` | `closeout-repair.mjs` parsing `phase-check CLOSEOUT` output | signed-scope artifact versus packet-declared `MERGE_BASE_SHA` and committed target | regenerate `signed-scope.patch` from `MERGE_BASE_SHA..COMMITTED_TARGET_HEAD_SHA`, then re-verify | mechanical if SHAs are sound | cheap when the packet is already coherent, impossible if upstream packet truth is wrong |
| `CLAUSE_COVERAGE_MISMATCH` | `closeout-repair.mjs` parsing `phase-check CLOSEOUT` output | packet clause matrix versus validation reports | no auto-fix; operator or validator must reconcile clause rows with actual reviewed clauses | judgment-bound | the kernel cannot infer clause intent from mismatched artifacts without risking false closure |
| `MISSING_VALIDATION_VERDICT` | `closeout-repair.mjs`, validator gate status, closeout truth sync | packet verdict block versus validator gate state | inspect gate state; if PASS is already committed, rely on closeout sync to project it into the packet, otherwise manually append and commit the validator gate verdict | mixed: probe is mechanical, resolution may still require lane action | this is usually a projection failure, not a review failure, but it still blocks final-lane progression |
| `COMMUNICATION_HEALTH` | `wp-communication-health-check`, surfaced through `phase-check CLOSEOUT` | receipts, thread state, pending route/notification residue | manually settle missing acknowledgements, receipts, and review-route residue, then re-run the health check | mostly manual | communication truth lives across multiple ledgers and is hard to normalize after drift |
| `INTEGRATION_VALIDATOR_CLOSEOUT` | `integration-validator-closeout-check` via `phase-check CLOSEOUT` | final-lane topology, actor identity, session-control bundle, signed-scope compatibility | inspect the exact preflight failure, repair lane/session/broker/state mismatches, then re-run closeout | mixed, often manual | this is a bundle failure class that compresses several control-plane invariants into one blocking verdict |

### Expanded Failure Families Hidden Inside `INTEGRATION_VALIDATOR_CLOSEOUT`

The catch-all closeout bucket is where most governance cost hides.
`integration-validator-closeout-lib.mjs` expands it into smaller failure families:

| Failure family | What is actually wrong | Current fix path | Why this tends to loop |
| --- | --- | --- | --- |
| Final-lane identity violation | closeout is running outside the Integration Validator lane, branch, or worktree | relaunch or resume the governed Integration Validator session in the correct lane/worktree | the failure is not in the review verdict; it is in who is allowed to author final truth |
| `HANDSHAKE_GOV_ROOT` / gov-root violation | final-lane logic resolved live governance from `handshake_main/.GOV` instead of the kernel | repair environment/root wiring before continuing | path drift is easy to create and hard to notice until late closeout |
| Missing durable committed evidence | committed handoff proof, target SHA, or durable PASS proof is absent | restore committed validation evidence before closeout | closeout cannot safely infer missing committed proof from live state alone |
| Blocking broker runs or unsettled request/result pairs | session-control requests, results, registry state, and broker active runs disagree | settle or cancel active runs, repair missing results/output logs, then re-check | ACP/session-control residue accumulates across retries and can make old state look current |
| Session registry/result mismatch | session says `RUNNING` or `COMPLETED`, but no matching settled result exists or statuses disagree | reconcile session registry and settled results | there are multiple ledgers for the same event, and any one of them can be stale |
| Signed-scope compatibility failure | recorded scope compatibility truth or candidate target validation no longer matches current main or packet claims | refresh compatibility truth, correct target/baseline claims, then re-run validation and closeout checks | repo movement and packet drift combine here, so old compatible truth can become false after the fact |

### What the Matrix Shows

- Closeout pre-repair is not mainly fixing code defects. It is fixing stale or divergent workflow truth.
- Only a small subset of failures are truly auto-repairable without risk.
- The highest-cost failures are bundle failures where several ledgers have to agree before the validator can write final-lane truth.
- The spec-level requirement is not optional convergence theater. It is authoritative transcription and completion safety.
- The redesign target is therefore not "remove closeout"; it is "make the convergence step execute against fewer, clearer, more canonical truth surfaces."

## Final-Lane Closeout Library

`integration-validator-closeout-lib.mjs` is where final-lane correctness becomes explicit.
Key exported functions include:

- `resolveCloseoutValidatorSessionsOfRecord`
- `deriveFinalLaneGovernanceInvalidity`
- `appendCloseoutSyncProvenance`
- `buildIntegrationValidatorCloseoutCheckResult`
- `formatIntegrationValidatorCloseoutCheckResult`

This library is trying to answer:

- is the final-lane actor really the Integration Validator of record?
- did closeout happen from the right worktree/root?
- is final-lane governance invalid because the wrong lane attempted the work?
- is the provenance of the closeout sync durable and attributable?

This is one repo-kernel realization of broader spec duties around authoritative transcription, lane-bound finalization, and completion safety.
The problem is not that this logic exists.
The problem is how much surrounding workflow state has to be correct before it can pass cleanly in the current kernel.

## Validator Runtime Artifacts

The validator subsystem keeps durable state in multiple places:

### Validator gate state

- `.GOV/roles_shared/runtime/validator_gates/<WP_ID>.json`
- `../gov_runtime/roles_shared/validator_gates/<WP_ID>.json`

These hold:

- validation session state
- archived sessions
- committed validation evidence
- closeout sync events

### Packet and communications surfaces

- work packet file
- runtime status file
- receipts and thread history

### Session and control surfaces

- session registry
- session-control request/result ledgers
- session-control outputs

Again, the cost problem is not a lack of state.
It is the number of overlapping state surfaces that must converge before the validator subsystem can move forward cleanly.

## Main Failure Hotspots in This Subsystem

### 1. Wrong-lane ambiguity

The system needs to know whether a write or gate action is being attempted from:

- WP Validator
- Integration Validator
- Orchestrator
- an invalid or unbound lane

When lane identity is unclear, gate writes must stop.

### 2. PASS depends on many upstream truths

PASS is not just validator judgment.
It depends on communication truth, evidence truth, final-lane truth, packet truth, and computed policy truth.

That means one stale upstream artifact can block final progression.

### 3. Closeout repair is both necessary and expensive

The existence of `closeout-repair.mjs` is a sign that the kernel identified a real failure mode and built a mechanical repair tool.
It is also a sign that closeout had become expensive enough to require a dedicated pre-repair stage.

### 4. Resume routing is inferential

`validator-next.mjs` is trying to infer the correct target WP and next action from multiple signals.
That is useful for continuity, but it also means validator routing depends on multi-surface workflow truth.

## What This Suggests for the Next Harness

The next harness likely needs:

- explicit validator-lane ownership in canonical workflow state
- a simpler PASS-clearance model with fewer overlapping truth dependencies
- stronger separation between review verdicts and closeout mechanics
- final-lane authority that is easier to prove mechanically
- fewer mutable surfaces that can block validator progression

## Evidence Anchors for This Topic

The next pass on this deep dive should repeatedly cite:

- `.GOV/roles/validator/scripts/validator-next.mjs`
- `.GOV/roles/validator/scripts/lib/validator-governance-lib.mjs`
- `.GOV/roles/validator/scripts/lib/integration-validator-closeout-lib.mjs`
- `.GOV/roles/validator/checks/validator_gates.mjs`
- `.GOV/roles_shared/checks/wp-communication-health-check.mjs`
- `.GOV/roles_shared/checks/phase-check.mjs`
- `.GOV/roles/orchestrator/scripts/closeout-repair.mjs`
- `.GOV/roles/validator/tests/*.mjs`
- `.GOV/Audits/smoketest/DOSSIER_20260413_CALENDAR_STORAGE_WORKFLOW_DOSSIER.md`
- `.GOV/Audits/smoketest/DOSSIER_20260414_DISTILLATION_WORKFLOW_DOSSIER.md`

## Next Deepening Pass

The next pass on this document should add:

- the validator gate-state JSON object model
- the exact resume-state decision tree for `validator-next.mjs`
- a PASS-clearance dependency matrix
- a closeout-repair failure-to-fix matrix
- a lane-authority matrix showing which role may write which artifact at which stage
