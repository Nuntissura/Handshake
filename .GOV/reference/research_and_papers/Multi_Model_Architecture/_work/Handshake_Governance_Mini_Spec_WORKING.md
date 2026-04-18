# Handshake Governance Mini Spec Working Draft

Temporary working draft that compresses the current research into a merge-oriented mini spec.

This is not yet a task packet and not yet master-spec text.
It is the smallest additive spec slice that:

- complies with current master-spec law
- translates the harness and repo-governance research into product terms
- can later be merged into the master spec without rewriting the whole research stack

## Purpose

- define the software-delivery governance overlay as an additive Handshake product capability
- state the minimum normative rules needed to implement it without violating current master-spec boundaries
- identify the concrete runtime records and projection behavior the product needs
- give a clean merge path into the master spec

## Compliance Posture

This draft is subordinate to `.GOV/spec/Handshake_Master_Spec_v02.180.md`.
It does not attempt to replace or fork existing law for:

- shared structured-collaboration base envelope
- compact summary contract
- workflow-state families, queue reasons, and governed-action descriptors
- Dev Command Center, Task Board, and Role Mailbox projection rules
- Governance Pack boundary rules
- Governance Check Runner bounded execution contract
- runtime governance storage boundary under `.handshake/gov/`

This draft is additive.
Where it introduces new terms, they should be interpreted as software-delivery overlay specializations that sit on top of the existing product substrate.

## Scope

In scope:

- software-delivery governance as one Handshake overlay profile
- product-owned runtime records for software-delivery governance state
- governed action, approval, validator, and projection behavior for that overlay
- migration of current repo-governance artifacts into imported overlay data, mirrors, evidence, or compatibility inputs

Out of scope:

- redefining all Handshake governance
- replacing project-agnostic workflow-state law
- treating repo `/.GOV/**` as live product runtime authority
- specifying the full implementation plan

## Core Thesis

Handshake should implement software-delivery governance as a product-native overlay with:

- imported overlay definition
- product-owned runtime truth
- product-owned bounded execution
- projection-only operator surfaces
- recorder-visible evidence and replay seams

The current repo-governance kernel remains:

- migration input
- failure evidence
- conformance reference for the software-delivery overlay

It does not remain the product authority model.

## Normative Clauses

### 1. Overlay boundary

- Handshake MUST treat software-delivery governance as an additive overlay profile, not as the whole governance kernel.
- Imported repo-governance artifacts MUST NOT replace or hide broader Handshake-native governance, structured-collaboration law, or project-profile contracts.
- Product runtime MUST NOT treat repo `/.GOV/**` as live execution authority.
- Runtime governance state for the overlay MUST live in product-owned storage, default `.handshake/gov/`.

### 2. Canonical runtime truth

- Software-delivery overlay state MUST resolve through canonical product-owned records rather than packet Markdown, board order, mailbox chronology, or side-ledger files.
- At minimum, the overlay SHOULD define canonical records for:
  - overlay binding
  - work contract
  - workflow binding
  - governed action request
  - governed action resolution
  - validator gate state
  - checkpoint lineage
  - evidence references
- These records MUST comply with the shared base-envelope and compact-summary rules already defined by the master spec.

### 3. Routing and action law

- The overlay MUST reuse the shared `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids` contract.
- The overlay MUST NOT invent a second routing vocabulary based on packet prose, board lanes, or mailbox order.
- Any overlay-specific labels or queue views MUST be implemented as project-profile or view-level relabeling over the existing routing law.
- State-changing software-delivery actions SHOULD resolve through governed-action descriptors, transition rules, queue automation rules, and executor-eligibility policies already expected by the master spec.

### 4. Work contract and execution identity

- Every software-delivery work item MUST have one canonical runtime contract identity separate from any readable packet filename or repo path.
- Human-readable work identifiers MAY remain visible, but runtime joins MUST resolve through canonical ids.
- Work contract state, workflow execution state, and session or claimant state MUST remain explicitly linked rather than reconstructed from thread or filesystem conventions.

### 5. Approval and deferred side effects

- Approval-bound or side-effecting software-delivery actions MUST be represented as durable governed-action request and resolution records.
- Approval, denial, retry, and unsupported outcomes MUST be queryable without transcript reconstruction.
- Mailbox replies, handoff summaries, and announce-back artifacts MUST NOT directly mutate canonical work state; they MUST resolve through governed action or explicit authoritative transcription.

### 6. Validator and check posture

- Software-delivery validator posture MUST be represented as a dedicated product runtime record family rather than hidden only in workflow binding or readable mirrors.
- That gate record MUST support both:
  - compatibility with legacy per-WP gate posture
  - bounded `CheckResult` rollups from `governance.check.run`
- Non-pass validation posture, blocked execution, unsupported checks, and supporting evidence MUST remain queryable from canonical runtime state.
- Imported checks MUST obey the additive overlay rule and the bounded execution rules already defined by the master spec.

### 7. Projection behavior

- Dev Command Center MUST remain the primary projection and control surface for the overlay.
- Task Board MUST remain a readable planning and synchronization projection, not a second source of execution authority.
- Role Mailbox MUST remain a collaboration and routing surface, not a linked work authority surface.
- Projection surfaces MUST expose reconciliation posture where mirror or summary drift exists.

### 8. Evidence and replay

- Overlay lifecycle events SHOULD be Flight-Recorder-visible at the same seams where authoritative state changes occur.
- At minimum, recorder-visible seams SHOULD include:
  - overlay activation
  - work contract activation or supersession
  - governed action request and resolution
  - validator gate transition and check execution outcome
  - checkpoint create, resume, and fork
  - authoritative handoff transcription
  - terminal closeout outcome
- Evidence records SHOULD reference recorder events and artifact handles rather than acting as workflow truth by themselves.

### 9. Migration and compatibility

- Current repo-governance artifacts SHOULD be classified before migration as one of:
  - imported overlay definition
  - canonical runtime truth target
  - projection or readable mirror
  - evidence
  - compatibility-only export or import surface
- Task packet Markdown, Task Board Markdown, runtime-status files, receipt streams, and thread artifacts MUST NOT remain equal-truth surfaces after migration.
- Legacy repo-governance artifacts MAY survive as mirrors, imports, or exports where required for continuity, but not as the long-term authority model.

## Proposed Record Set

This draft assumes the following record families are the smallest coherent product slice:

- `GovernanceOverlayBinding`
- `GovernanceWorkContractRecord`
- `GovernanceWorkflowBindingRecord`
- `GovernedActionRequest`
- `GovernedActionResolution`
- `GovernanceValidatorGateRecord`
- `GovernanceCheckpointRecord`
- `GovernanceEvidenceRecord`

These names are still working names, but the record roles should remain stable.

## Candidate Merge Targets In Master Spec

This draft is best merged as additive clarifications or new paragraphs near existing law, not as an isolated parallel chapter.

Primary merge targets:

- Governance Pack and product-governance boundary sections
- Governance Check Runner bounded execution section
- structured collaboration base-envelope and compact-summary sections
- workflow-state, governed-action, transition-rule, and executor-eligibility sections
- Dev Command Center / Task Board / Role Mailbox backend and projection sections

Merge intent:

- reuse existing law whenever possible
- add software-delivery overlay specialization where the current spec is still too generic
- avoid duplicating project-agnostic contracts already defined elsewhere

## Build Consequence

If this mini spec is accepted, implementation should start from product runtime truth, not from repo-file orchestration.

The first buildable slice is:

1. canonical work and workflow-binding records
2. governed action request and resolution
3. validator gate records
4. DCC-facing projection over those records
5. compatibility import and mirror flows for current repo-governance artifacts

## Why This Is Mergeable

This draft is mergeable because it does not fight the master spec.
It assumes:

- one product substrate
- one routing law
- one bounded execution contract
- one projection model
- one runtime boundary

The research contribution is only to say how software-delivery governance should use those existing product laws.

## Current Use

Use this draft as the bridge from research to master-spec editing.
It is the document to consult when converting the working notes into:

- exact master-spec clause insertions
- term alignment against existing primitives
- implementation-ready packet refinement later
