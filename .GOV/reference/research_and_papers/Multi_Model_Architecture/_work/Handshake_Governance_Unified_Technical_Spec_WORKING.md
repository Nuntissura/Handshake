# Handshake Governance Unified Technical Spec Working Draft

Temporary working draft that merges:

- `Handshake_Governance_Mini_Spec_WORKING.md`
- `Handshake_Multi_Agent_Swarm_Harness_Unified_Research_and_Technical_Spec_v1.0.md`

This document does not modify the Master Spec.
It is the merged working source to use before any future main-body merge.

## Purpose

- preserve the mini spec's master-spec compliance posture
- preserve the external draft's deeper runtime and control-plane detail
- give one consolidated workspace document for later clause extraction, refinement, and implementation planning

## Source Role

Use this document as:

- the unified technical source for the software-delivery governance overlay
- the intermediate bridge between research notes and future master-spec edits
- the comparison-resolved baseline for future implementation packets

Do not use this document as:

- a replacement for `.GOV/spec/Handshake_Master_Spec_v02.180.md`
- a source of live product authority

## Compliance Posture

This draft is subordinate to `.GOV/spec/Handshake_Master_Spec_v02.180.md`.
It is intentionally additive and assumes the current master-spec law for:

- shared structured-collaboration base envelope
- compact summary contract
- project-agnostic workflow-state families, queue reasons, and governed actions
- Dev Command Center, Task Board, and Role Mailbox projection boundaries
- Governance Pack overlay boundary
- Governance Check Runner bounded execution
- product-owned runtime governance storage under `.handshake/gov/`

Where this draft adds detail, that detail should be interpreted as software-delivery overlay specialization on top of the existing product substrate.

## Scope Boundary

In scope:

- the Handshake-native software-delivery governance overlay
- authoritative runtime records for software-delivery governance state
- approval, validator, checkpoint, ownership, and projection semantics for that overlay
- migration from the current repo-governance kernel into product-native runtime truth

Out of scope:

- all Handshake governance for every future project profile
- raw porting of the current repo-governance shell into the product runtime
- provider-specific model APIs
- final master-spec wording

## Executive Synthesis

### Core conclusion

Handshake should not grow by expanding the current repo-governance shell into a larger shell.
It should implement a product-owned governance control plane with:

- imported software-delivery overlay definitions
- product-owned runtime truth
- typed workflow, approval, validator, and checkpoint objects
- bounded execution and bounded parallelism
- projection-only operator surfaces
- recorder-visible evidence and replay

### Main architectural move

The center of the design is:

- imported software-delivery governance defines the overlay
- Handshake runtime owns the truth

### Main things worth preserving from the current kernel

- bounded work contracts
- explicit workflow authority versus technical authority
- layered validator split
- evidence discipline
- mechanical checks
- manual relay as a real fallback mode

### Main things that must be replaced

- too many truth surfaces
- packet-as-contract plus packet-as-ledger overload
- session truth that is durable but not convergent enough
- closeout that is correct but too repair-heavy
- routine success depending too much on expert operator intervention

## Normative Principles

### 1. Authoritative truth principle

Every live software-delivery workflow fact MUST have one authoritative home in product runtime records.

### 2. Projection discipline principle

Dev Command Center, Task Board, Role Mailbox, readable Markdown, compatibility views, and dashboards MUST remain derived or controlled surfaces, not alternate truth stores.

### 3. Explicit transition principle

Important workflow transitions MUST be explicit runtime transitions, not narrative guesses or board-motion heuristics.

### 4. Approval-as-data principle

Any action crossing a capability, policy, or authority boundary MUST be represented as typed request and resolution state.

### 5. Evidence-not-transcript principle

Replay, audit, and finalization MUST rely on structured evidence and event records, not transcript archaeology alone.

### 6. Same-state-model principle

Manual relay and autonomous orchestration MUST act on the same authoritative state model.

### 7. Bounded execution principle

Imported overlay execution MUST pass through product-owned capability and check-runner boundaries.

### 8. Identity proof principle

Final authority writes MUST be attributable to the correct role, lane, session, and claim or lease posture.

## Architecture Overview

### Layer 1: Product runtime authority

Owns live software-delivery truth:

- workflow execution truth
- governed action truth
- capability and approval truth
- checkpoint truth
- validator gate truth
- claim or lease truth where enabled

### Layer 2: Overlay definition

Owns imported overlay definitions:

- codex descriptors
- protocol descriptors
- rubric descriptors
- check descriptors
- template descriptors
- schema descriptors
- provenance and compatibility metadata

### Layer 3: Governed execution

Owns bounded execution:

- imported checks
- rubric evaluation
- side-effecting overlay actions
- merge or export or closeout support
- capability-gated and approval-gated execution

### Layer 4: Projection

Owns operator-facing surfaces:

- Dev Command Center
- Task Board
- Role Mailbox
- readable mirrors
- compatibility projections

### Layer 5: Evidence

Owns:

- Flight Recorder seams
- evidence bundles
- replay bundles
- command journals as evidence
- export records

## Workflow-State and Routing Law

The overlay MUST reuse the existing master-spec routing substrate:

- `workflow_state_family`
- `queue_reason_code`
- `allowed_action_ids`

The overlay MUST NOT invent a second routing vocabulary based on:

- packet prose
- board lane names
- mailbox order
- filesystem conventions

Overlay-specific labels MAY exist, but only as relabeling over the shared routing law.

## Execution Profiles

The overlay SHOULD distinguish work classes so routing, retry budgets, and governance posture do not collapse into one generic path.

Recommended profiles:

- `deterministic_strict`
- `heuristic_exploratory`
- `refinement_discovery`
- `validation_heavy`
- `operator_supervised`

These profiles are routing and execution-policy inputs, not alternate workflow-state families.

## Core Record Families

### Core v1 record set

The smallest coherent v1 record set is:

- `GovernanceOverlayBinding`
- `GovernanceWorkContractRecord`
- `GovernanceWorkContractSummary`
- `GovernanceWorkflowBindingRecord`
- `GovernedActionRequest`
- `GovernedActionResolution`
- `GovernanceValidatorGateRecord`
- `GovernanceCheckpointRecord`
- `GovernanceEvidenceRecord`

These records MUST remain compliant with the shared base envelope and compact summary law already owned by the master spec.

### Extension candidates

The external draft adds two useful but not yet mandatory record families:

- `GovernanceClaimLeaseRecord`
- `GovernanceQueuedInstructionRecord`

These should be treated as extension candidates unless and until they are explicitly promoted into the next master-spec pass.

## Canonical Runtime Semantics

### Work contract

The work contract is the canonical bounded work meaning.
It replaces packet Markdown as the mutable operational ledger.

Required semantics:

- one canonical runtime contract id
- explicit authority assignment
- scope and done-means
- overlay binding
- summary link
- source-artifact references

### Workflow binding

The workflow binding connects the contract to live execution.

Required semantics:

- workflow run id
- active node execution ids
- current session ids
- next expected actor
- transition and eligibility policy refs
- linked validator gate refs
- retry budget
- health posture

### Governed action request and resolution

These are the durable stop-and-resume envelopes for side effects and approvals.

Required semantics:

- one stable action request id
- explicit target action identity
- policy and capability basis
- request status
- resolution kind and resolution actor
- recorder-visible execution or denial outcome

### Validator gate record

Validator posture must be a first-class runtime record, not hidden in workflow binding or readable mirrors.

Required semantics:

- gate scope
- gate phase
- compatibility gate status
- bounded `CheckResult` rollup
- validation session refs
- check-result refs
- pass-authority proof where needed

### Checkpoint record

Checkpoints must provide restart-safe lineage and forkable recovery.

Required semantics:

- immutable checkpoint identity
- parent checkpoint
- restoration compatibility
- explicit restore and fork visibility

### Evidence record

Evidence records are authoritative about evidence references, not about workflow truth itself.

Required semantics:

- event refs
- artifact refs
- provenance status
- summary-first access path

## State and Lifecycle Model

### Work contract lifecycle

Allowed path:

- `intake -> ready -> active -> waiting/review/approval/validation/blocked -> done/canceled -> archived`

Critical invariants:

- a work contract MUST NOT be `done` while required validator gates remain uncommitted
- a work contract MUST expose explicit blockers when in `blocked`
- a work contract MUST NOT jump from `intake` directly to `done`

### Workflow binding lifecycle

Recommended binding states:

- `created`
- `queued`
- `claimed`
- `node_active`
- `approval_wait`
- `validation_wait`
- `closeout_pending`
- `settled`
- `failed`
- `canceled`

Critical invariants:

- at most one `node_active` binding per work contract unless explicit fan-out policy permits otherwise
- `approval_wait` requires unresolved governed actions
- `validation_wait` requires active gate records
- `closeout_pending` is derived from runtime truth, not packet prose

### Governed action lifecycle

Recommended statuses:

- `pending_policy`
- `pending_approval`
- `approved`
- `executing`
- `executed`
- `denied`
- `retry_requested`
- `skipped`
- `unsupported`
- `expired`

Critical rules:

- resumption MUST match `action_request_id`
- approval MUST produce a resolution record
- terminal request states remain terminal unless explicitly superseded by a new request

### Validator gate lifecycle

Recommended gate phases:

- `pending`
- `presented`
- `acknowledged`
- `appending`
- `committable`
- `committed`
- `archived`

Critical PASS rule:

- PASS is not authoritative on its own
- final PASS requires a committable or committed gate, explicit authority proof, and required evidence

### Checkpoint lifecycle

Rules:

- checkpoints SHOULD be immutable once written
- restore MUST be recorder-visible
- forks MUST preserve lineage

### Claim or lease lifecycle

If claim or lease becomes first-class, it should govern:

- temporary ownership
- takeover legality
- lease expiry
- final authority proof for sensitive actions

### Queued instruction lifecycle

If queued instruction becomes first-class, it should govern:

- steer-next and follow-up injection
- safe barriers for active runs
- recorder-visible queued-versus-injected state

## Control-Plane Model

The future control plane should keep the shape of the current session-control surfaces:

- start
- steer
- cancel
- close
- recover

But runtime truth must move into canonical runtime objects instead of being inferred from request and result ledgers alone.

### Required control-plane behaviors

- canonical start sequence
- safe steer-next / queued instruction path
- explicit cancel sequence
- explicit close sequence
- recovery and reattach or checkpoint restore path
- health posture and stale detection
- backpressure instead of silent drop
- push-oriented alerting outside the active terminal

## Validator and Closeout Model

### Preserve the layered validator split

Keep:

- WP Validator for local/per-MT technical review
- Integration Validator for whole-WP final automated authority

### PASS authority

A PASS becomes authoritative only when:

- the gate record is committable or committed
- required evidence exists
- required role, session, and claim or lease posture are proven
- closeout is not blocked

### Closeout as derived convergence

Closeout must be computed from:

- work contract
- workflow binding
- validator gate records
- governed action resolutions
- evidence bundles
- claim/session posture where applicable
- repo containment or compatibility proof

Closeout MUST NOT depend on packet surgery to become true.

## Projection Model

### Dev Command Center

DCC is the canonical control-plane surface for:

- work contract state
- workflow binding state
- model session state
- validator gate posture
- pending approvals
- route and claim posture
- checkpoint lineage
- evidence and replay

### Task Board

Task Board is a planning and synchronization mirror over authoritative runtime records.
It MUST NOT become the authority for:

- workflow state family
- lane legality
- closeout truth
- next-action legality

### Role Mailbox

Role Mailbox is for collaboration and routing.
It may surface messages, handoffs, waits, and announce-back information.
Changes to authoritative work meaning MUST go through governed transcription.

### Readable Markdown

Readable packet and board mirrors remain useful for export, scanning, review, and compatibility.
They MUST expose reconciliation posture and must not silently outrank canonical state.

## Evidence and Replay

Recorder and evidence visibility should exist for at least:

- overlay binding activation
- contract activation and supersession
- workflow binding activation
- governed action request creation
- approval, denial, and execution resolution
- validator gate lifecycle transitions
- checkpoint create, restore, and fork
- claim or lease grant/release where applicable
- queued instruction injection where applicable
- session start, steer, cancel, close, and recover
- terminal closeout outcome

Evidence bundles SHOULD be summary-first and drilldown-capable.
Replay bundles SHOULD reconstruct the contract of record, the workflow binding of record, governed-action history, validator-gate history, and key evidence refs.

## Imported Overlay and Bounded Execution

Imported overlay assets are:

- codex
- protocols
- rubrics
- checks
- templates
- schemas

They are imported definitions, not live workflow truth.

Imported execution MUST route through product-owned capability-gated and recorder-visible execution.
Imported scripts MUST NOT become default executable authority just because they exist in repo governance.

For risky side effects, the runtime SHOULD support copy-on-execute or similarly bounded pre-commit execution patterns.

## Migration Model

The current repo-governance kernel should be migrated by classification, not by direct runtime reuse.

Every current artifact should be classified as:

- imported overlay definition
- canonical runtime truth target
- projection or readable mirror
- evidence
- compatibility-only import or export surface

Concrete migration direction:

- packets become source artifact plus readable mirror over work contract truth
- validator gate files become compatibility imports into validator gate records
- session control ledgers become evidence and compatibility command journals
- Task Board and runtime status become projections
- dossier summaries become generated evidence views

## Conformance and Implementation Posture

The research suggests a failure-class-based conformance harness is worth keeping.
At minimum, the system should be able to prove protection against:

- truth authority drift
- unsafe resume
- claim or ownership ambiguity
- PASS without attributable authority
- closeout convergence failure
- imported execution overreach

## Recommended Implementation Sequence

1. canonical authority store for work, workflow binding, governed action, validator gate, checkpoint, and evidence
2. runtime convergence for session and health posture
3. governed actions and approvals
4. validator and closeout hardening
5. projection cutover in DCC, Task Board, and Role Mailbox
6. bounded swarm semantics such as claims, leases, and queued instruction injection where justified

## Candidate Merge Targets In The Master Spec

This unified draft still should not be merged wholesale.
It is a source document for selective extraction into:

- Governance Pack and product-governance boundary sections
- Governance Check Runner bounded execution section
- structured collaboration base-envelope and compact-summary sections
- workflow-state, governed-action, transition-rule, and executor-eligibility sections
- Dev Command Center, Task Board, and Role Mailbox projection sections
- governance-related appendix and matrix updates

## Working Decision

For future master-spec work:

- use this document as the deepest merged technical source
- use `Handshake_Governance_Mini_Spec_WORKING.md` as the smallest merge skeleton when doing actual main-body edits
- use `Handshake_Governance_Master_Spec_Insertion_Plan_WORKING.md` as the section-by-section patch map against the current master spec
- treat the current repo-governance kernel as migration corpus and conformance suite, not the final runtime architecture
