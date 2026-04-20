# Workflow State, Packet Truth, and Range Drift

## Purpose

This document is the deep dive on the most important failure class observed so far:
the system losing agreement about workflow truth.

This includes:

- packet truth drift
- stale merge-base truth
- stale candidate range truth
- placeholder packet sections blocking later phases
- false out-of-scope diffs
- closeout truth requiring repair before validators can judge the real work

The goal is to explain why small product diffs can become large governance incidents.

## Spec-Defined Authority Posture

The current Master Spec already defines the target direction for workflow truth.
The important anchors are:

- Task Board synchronization state and related planning artifacts are canonical backend coordination state; human-readable Task Board views are synchronized mirrors and must not become a second execution authority.
- Canonical structured JSON or JSONL collaboration records are the executable authority for routing, validation, and readiness state.
- Work Packets are authoritative execution contracts.
- Mailbox handoff or announce-back that changes linked work meaning must transcribe into authoritative artifacts; mailbox narrative alone cannot become durable final state.

That matters because this document is about current-kernel truth drift, not about redefining authority from scratch.
Much of the current kernel cost appears to come from living between the spec's structured-record target and a still-heavy packet, mirror, and runtime-ledger implementation.

## Relationship to Other Research Documents

| Document | Role |
|---|---|
| `Gov_Kernel_Technical_Map.md` | Whole-system map |
| `ACP_Broker_and_Session_Control.md` | Launch and session-control deep dive |
| `Workflow_State_Packet_Truth_and_Range_Drift.md` (this file) | Deep dive on workflow truth and false failure mechanics |
| `Harness_Lessons_Learned.md` | Cross-cutting lessons and implications |

## Core Thesis

The current kernel's dominant cost center is not ordinary implementation difficulty.
It is workflow truth drift around the implementation.

In practical terms:

- product work can be small and correct
- validators can still be blocked
- phase checks can still fail
- out-of-scope diffs can still be reported
- the orchestrator can still spend large amounts of time repairing metadata and control-plane truth

## Truth Surfaces That Must Agree

The kernel currently relies on several truth surfaces agreeing at the same time:

### Authored workflow truth

- refinement documents
- task packet fields
- task board state
- signature and role-model selections

### Runtime workflow truth

- runtime status files
- governed receipts and thread history
- role session registry
- session control request/result ledgers

### Mechanical validation truth

- `phase-check` outputs
- packet truth checks
- packet closure monitoring
- merge progression truth
- closeout repair output

### Audit truth

- live workflow dossier
- fail captures
- governance memory

The system slows down when one or more of these disagree.

## Truth-Surface Matrix

The current kernel does not have one truth surface.
It has a stack of authored, projected, runtime, and audit surfaces that must stay aligned.

| Truth surface | Primary writer(s) | Primary reader(s) | What it is supposed to mean | Common drift mode | Current repair path |
|---|---|---|---|---|---|
| Work packet file in `.GOV/task_packets/...` | Activation Manager initially; later orchestrator, coder, validators, and closeout repair flows | `phase-check`, `packet-truth-check`, validators, orchestrator, runtime projection, audit review | Canonical authored workflow contract: status, scope, branch/worktree declarations, validator-of-record, compatibility truth, containment truth, evidence sections | Placeholder sections remain; status and range truth lag behind reality; merge-base or containment fields become stale | Manual packet edits, orchestrator repair, `just closeout-repair`, then re-run `just phase-check ...` |
| Task board entry in `.GOV/roles_shared/records/TASK_BOARD.md` | Orchestrator/governance projection flows | Orchestrator startup/next, governance checks, humans scanning state | Repo-level compressed projection of WP state | Token does not match packet status, active packet missing, older packet not marked superseded | Resync packet/task-board projection; rerun `packet-truth-check`; manual board correction if necessary |
| Traceability registry in `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md` | Governance packet/registry maintenance flows | `packet-truth-check`, orchestrator, humans | Declares which packet path is active for a base WP | Active packet path points at missing or wrong packet; base WP mismatch | Repair active packet mapping; rerun `packet-truth-check` |
| Runtime status file under `WP_COMMUNICATIONS/<WP>/RUNTIME_STATUS.json` | Runtime projection/sync flows; communications updates | Orchestrator, validators, dossier sync, packet-runtime-projection drift checks | Machine-readable current phase, milestone, next actor/session, and runtime-side view of packet truth | Runtime packet status, task-board status, or containment fields lag behind packet truth | Re-run runtime projection sync; repair packet truth first if packet is wrong; then re-check drift |
| Governed receipts and thread in `WP_COMMUNICATIONS/<WP>/RECEIPTS.jsonl` and `THREAD.md` | Coder, validators, orchestrator through governed receipt surfaces | Validators, orchestrator, dossier, communication-health checks | Durable record of who said what and which role owns the next move | Missing kickoff/handoff/review response; direct review completed but runtime/packet still show earlier phase | Append missing receipt, resync runtime interpretation, rerun communications and phase checks |
| Role session registry in `ROLE_SESSION_REGISTRY.json` | Session-control command path and registry mutation helpers | Orchestrator, health tools, session governance checks, dossier sync | Durable session identity, runtime state, thread id, last command state, host ownership | Session marked running/ready/closed inconsistently with real ACP or terminal state | Session-control recovery, self-settle, terminal reclaim, re-steer or close session |
| Session control request/result ledgers in `SESSION_CONTROL_REQUESTS.jsonl` and `SESSION_CONTROL_RESULTS.jsonl` | `session-control-command.mjs`, broker, self-settle logic | Orchestrator, runtime health tooling, dossier sync, token ledger sync | Durable command trail for start/steer/cancel/close operations | Missing terminal result row, stale active run, cancel request without converged target result | `settleRecoverableSessionControlResults`, wait/recover, rerun command if needed |
| Gate outputs in `GATE_OUTPUTS/` and validator gate files | `phase-check`, validator checks, packet-truth checks | Orchestrator, validators, dossier, humans diagnosing failures | Deterministic statement about whether workflow truth passes a given gate | Gate reasons over stale range, stale packet field, or stale runtime projection and fabricates false workflow failure | Repair underlying truth surface, then rerun gate; do not treat gate output as self-healing truth |
| Merge progression / containment truth in packet plus merge libs | Packet updates, closeout repair, merge containment sync | `merge-progression-truth-lib`, closeout, runtime projection, validators | Whether work is merge pending, contained in main, or not required | `MAIN_CONTAINMENT_STATUS`, merged main SHA, or baseline SHA lags behind actual repo state | `just closeout-repair`, explicit SHA update, rerun closeout gate |
| Live workflow dossier in `.GOV/Audits/smoketest/...` | Orchestrator and sync/autofill flows | Humans, later research, smoketest review | Human-readable run history and failure ledger | Placeholder-heavy dossier lags the actual run, forcing manual reconstruction | `workflow-dossier-sync`, `workflow-dossier-note`, selective manual repair |

## How to Read the Matrix

Three structural problems stand out:

### 1. The packet is both contract and mutable ledger

The packet is not only the signed work contract.
It is also used as a mutable status ledger, containment ledger, evidence ledger, and validator ledger.

That makes it powerful, but also makes it a high-friction truth surface.
Many failures happen because later checks expect the packet to already reflect runtime reality.

This is a current-kernel implementation reality, not the clean target shape implied by the spec's structured-record-first direction.

### 2. Several surfaces are projections, not true authorities

The task board, runtime status file, and parts of the dossier are projections.
They should agree with upstream truth, but they do not own that truth.

When they disagree, the system often forces the operator or orchestrator to determine:

- is the projection stale?
- is the packet stale?
- is the runtime stale?
- or did the underlying work actually fail?

### 3. Gate outputs are verdicts on truth, not truth themselves

This is a key distinction.
`phase-check` and related gates are readers and judges.
They are not the primary source of workflow truth.

When a gate fails because it read stale upstream truth, the system can waste large amounts of time debugging the verdict instead of repairing the source.

## Anchor Case: WP-1-Calendar-Storage-v2

The calendar-storage run is the first strong anchor case for this failure class because it has both:

- concrete patch artifacts showing wrong-target versus correct-target diff scope
- a dossier that records the recovery sequence in detail

Primary evidence:

- `.GOV/Audits/smoketest/DOSSIER_20260413_CALENDAR_STORAGE_WORKFLOW_DOSSIER.md`
- `.GOV/Audits/smoketest/WP-1-Calendar-Storage-v2-MISROUTED_MAIN_DIFF-20260413T123133Z.patch`
- `.GOV/Audits/smoketest/WP-1-Calendar-Storage-v2-CANDIDATE_TARGET-066cc18d.patch`

## Failure Pattern Seen in the Calendar-Storage Run

The dossier shows a clear pattern:

1. The actual product diff was narrowed to a small contained change surface.
2. Mechanical workflow truth still evaluated an older or wrong range.
3. `phase-check HANDOFF` kept reasoning over stale provenance.
4. Placeholder packet sections also caused mechanical failures.
5. The coder stalled on governance recovery rather than advancing product work.
6. The orchestrator had to cancel, re-steer, repair packet truth, and re-run deterministic checks.
7. Only after truth resynchronization could the governed handoff and validator loop proceed cleanly.

This is a control-plane incident, not a product-quality incident.

## Calendar-Storage Recovery Walkthrough

This is the first concrete walkthrough that should anchor later generalizations.
The important point is not just that recovery happened.
The important point is which parts of the recovery were product work and which parts were truth repair.

### Step 1: Product scope was already narrow

The dossier records that the aligned candidate surface was small:

- the branch was rebuilt into a clean linear WP branch
- the intended implementation range became `099f004d..HEAD`
- the final contained candidate was `066cc18d`
- the real diff versus `origin/main` collapsed to 4 in-scope storage files

This matters because it disproves the idea that the workflow was slow because the implementation itself was large.

### Step 2: Mechanical handoff truth was still wrong

Even after branch cleanup, the handoff gate continued to evaluate the wrong range.
The dossier records a stale evaluation against `facce56..d0832fe0`, which produced false out-of-scope results.

This is the core failure:

- product scope had been corrected
- gate scope had not

### Step 3: Placeholder packet fields amplified the failure

The same handoff path also failed because packet sections such as:

- `VALIDATION`
- `STATUS_HANDOFF`
- `EVIDENCE_MAPPING`
- `EVIDENCE`

were still placeholders or incomplete.

This created a compound failure mode:

- stale candidate range
- incomplete packet truth

Either one could block progress.
Together they made the gate output look much worse than the product reality.

### Step 4: The coder stalled on governance recovery

The coder did not fail because the storage change was unsound.
The coder stalled on gate-log inspection and governance-side diagnosis.

This is one of the most expensive signs of a bad harness:

- the implementation role is spending its scarce cycles on control-plane archaeology

### Step 5: The orchestrator had to intervene mechanically

The dossier shows a specific orchestrator recovery pattern:

1. inspect the failing gate situation
2. cancel the stuck coder run
3. re-steer the coder lane
4. diagnose stale packet merge-base truth and missing handoff evidence
5. force packet/worktree truth resynchronization
6. re-run deterministic handoff checks

This is valuable as a recovery capability.
It is also evidence that the current common path still depends on orchestrator repair skill.

### Step 6: Truth resynchronization unblocked the real workflow

After packet/worktree truth was synchronized:

- `phase-check HANDOFF` passed on the intended `099f004d..d0832fe0` range
- governed `CODER_HANDOFF` appended successfully
- validator review resumed on the actual candidate instead of on stale provenance

This is the strongest evidence that the earlier blockage was governance drift, not product failure.

### Step 7: Validation then found a real product issue

Once truth was repaired, the validator surfaced a real compile problem on `cfd7a388`:

- `CalendarEventVisibility::Default` in `storage/tests.rs`

That distinction matters.
The harness only became useful again after the false control-plane failures were removed.
Then it could do its intended job and catch a genuine code problem.

### Step 8: Closeout also required truth repair

The dossier then records another repair step before final convergence:

- packet truth had to be repaired with the literal coder handoff range
- validator-gate recovery had to restore durable committed-target proof
- only then could `phase-check CLOSEOUT` pass in `CONTAINED_IN_MAIN` mode

So the same failure class appeared twice:

- once at handoff
- once again at closeout

## Why This Walkthrough Matters

This walkthrough is important because it shows the exact shape of the current bottleneck:

- a small product diff
- a larger governance recovery incident
- a temporary inability to distinguish false failures from real failures
- successful recovery only after truth surfaces were manually re-aligned

That is the pattern the next harness must eliminate.

## Implications for the Next Harness

This matrix suggests a more precise redesign target:

### 1. One canonical workflow-state object

The future harness should have one durable state object for workflow truth and candidate-range truth.
Packet, task board, runtime status, and much of dossier state should become projections from that object.

### 2. Clear separation between contract truth and execution truth

The packet should remain the contract artifact.
But execution-state mutation should move into a dedicated state model rather than continuously rewriting packet sections as the operational ledger.

### 3. Drift detection should point to the upstream owner immediately

A good harness should say:

- packet is stale
- runtime projection is stale
- task board projection is stale
- session registry is stale

instead of emitting a generic closeout or handoff failure that forces human diagnosis.

## Why This Failure Class Is So Expensive

Workflow truth drift is expensive because it compounds:

### It fabricates extra work

False out-of-scope diffs and stale range evaluation make the system "discover" failures that are not in the real implementation surface.

### It stalls the most expensive roles

Coder, validator, and orchestrator time all get spent on reconciling state instead of progressing the WP.

### It multiplies governance artifacts

Every repair tends to require more:

- packet edits
- gate re-runs
- registry inspection
- runtime output inspection
- dossier updates
- operator interpretation

### It reduces trust in the harness

Once the operator knows the harness can be wrong about the candidate range or packet truth, every failure becomes suspect and manual verification increases.

## Specific Drift Modes to Document

The next pass should separate at least these drift modes:

### 1. Packet content drift

The packet's sections and status fields are not aligned with actual workflow progress or actual implementation range.

### 2. Merge-base drift

The candidate diff is compared against the wrong baseline, making contained work look wide.

### 3. Range-selection drift

The deterministic check reasons over an older commit range than the real candidate range.

### 4. Placeholder-section drift

Packet placeholders remain in sections that later checks treat as blocking workflow truth.

### 5. Closeout-truth drift

The product work is effectively done, but closeout cannot converge because packet, merge, validator, and runtime truth are not synchronized.

## Current Kernel Strengths in This Area

The kernel does have the right instinct:

- keep explicit packets
- run deterministic checks
- record merge progression truth
- force closeout discipline

The failure is not that truth is checked.
The failure is that too much truth is partially authored, partially inferred, and repaired after drift instead of being derived from one canonical state model.

## Design Consequences for the Next Harness

The next harness should strongly consider:

### One canonical execution-state object

There should be one durable state object for:

- current workflow phase
- active MT
- approved scope
- baseline commit
- candidate range
- merge containment state
- current validator owner
- repair state

Other artifacts should derive from this object rather than restating it manually.

### Derived packet truth where possible

The packet should remain important, but many fields should be projections from canonical workflow state, not hand-maintained truth islands.

### Immutable candidate-range records

Once a candidate execution range is established for a handoff, the runtime should preserve that range explicitly and checks should consume it directly.

### Explicit truth-repair protocol

When truth divergence is detected, the system should enter a defined reconcile state instead of relying on ad hoc orchestrator repair.

### Manual relay on the same state model

If the operator needs to step in, manual relay should use the same workflow state object and not fork reality into a second undocumented operating mode.

## Evidence Anchors for the Next Pass

The next pass should quote and analyze:

- timeline entries in `DOSSIER_20260413_CALENDAR_STORAGE_WORKFLOW_DOSSIER.md`
- failure findings in that dossier, especially command-surface drift and packet truth drift
- the misrouted patch versus corrected candidate patch
- `phase-check` implementation and `phase-check-lib.mjs`
- packet-truth and packet-closure-monitor checks
- merge progression truth helpers

## Questions to Answer Through External Research

1. How do stronger harnesses preserve canonical task state across retries, branch surgery, and validation?
2. Which systems treat packets or task specs as immutable contracts versus generated projections?
3. How do state-machine-driven systems preserve creative work while still preventing truth drift?
4. What is the best way to represent candidate range truth for multi-branch or multi-agent work?
5. Which parts of closeout should be impossible to do manually because they should always be derived?

## Next Deepening Pass

The next pass on this document should add:

- a truth-surface matrix showing source of truth, writer, reader, failure mode, and repair path
- a list of packet fields that should become derived fields
- a proposed canonical workflow-state schema for a future harness
