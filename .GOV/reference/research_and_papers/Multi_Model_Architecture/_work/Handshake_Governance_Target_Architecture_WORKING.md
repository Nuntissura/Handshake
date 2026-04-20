# Handshake Governance Target Architecture Working Note

Temporary working note for the target architecture that should replace the current repo-governance testbed as the long-term design reference.

This is not an implementation packet.
It is the architecture synthesis that sits after:

- repo-governance failure inspection
- external harness mechanism extraction
- product-translation bridge in `.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Handshake_Governance_Product_Translation_WORKING.md`
- merge-oriented mini spec draft in `.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Handshake_Governance_Mini_Spec_WORKING.md`
- merged unified technical draft in `.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Handshake_Governance_Unified_Technical_Spec_WORKING.md`
- product-governance boundary work
- field-level schema refinement in `.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Handshake_Governance_Runtime_Schema_Sketch_WORKING.md`

## Purpose

- define the Handshake-native architecture for the software-delivery governance overlay
- make clear which runtime objects are authoritative
- make clear which current repo-governance artifacts become projections, evidence, or imported overlay data
- preserve the strong parts of current repo governance without preserving its fragmented truth model

## Scope Boundary

This note is about the software-delivery governance overlay inside Handshake.

It is not trying to define all Handshake governance for every creative or project kernel.
It assumes:

- Handshake is broader than repo governance
- repo governance remains an important software-delivery testbed
- the software-delivery overlay must live inside product-owned runtime and projection surfaces
- repo `/.GOV/` remains the repo-side governance workspace and canonical export or snapshot source, while product runtime state lives in product-owned storage such as `.handshake/gov/`

## Core Thesis

The next architecture should not treat repo governance as a set of live files plus runtime side ledgers.
It should treat software-delivery governance as:

- imported overlay policy and artifact definitions
- product-owned workflow state
- product-owned capability-gated execution
- product-owned projections and evidence

In short:

- imported governance artifacts define what the software-delivery overlay means
- Handshake runtime defines what is currently true
- Handshake projections define what operators see
- Flight Recorder defines what can be audited and replayed

## Inputs To This Architecture

- current repo-governance failure evidence:
  - truth authority drift
  - session-control residue
  - closeout convergence failure
  - command-surface drift
  - handoff routing residue
  - high inspection and repair cost
- external harness mechanisms worth borrowing:
  - `LangGraph` durable run state, interrupt/resume, lineage
  - `Open SWE` deterministic execution identity and queued follow-up injection
  - `PydanticAI` deferred request/result envelopes
  - `Letta` policy-as-data and durable approval stops
  - `AutoGen` versioned state envelopes and typed event vocabulary
  - `CrewAI` checkpoint lineage and fork/resume semantics
  - `BeeAI`, `Cline`, `Roo Code`, `smolagents`, `TaskWeaver`, `SWE-agent` for targeted policy, routing, replay, and artifact lessons

## Architectural Goals

1. one authoritative workflow-state family for software-delivery governance
2. one authoritative session and execution identity model
3. one product-owned approval and governed-action model
4. one clean split between imported overlay definition and live runtime state
5. projection-first operator surfaces instead of hand-maintained truth duplication
6. recorder-visible lifecycle and evidence for replay and audit
7. one state model that supports both autonomous orchestration and degraded manual relay

## Non-Goals

- directly importing repo `/.GOV/**` as live product authority
- preserving packet, task board, runtime status, and ledgers as equal truth surfaces
- rebuilding repo-governance shell choreography inside the product
- making the software-delivery overlay the whole of Handshake governance

## Architecture Layers

### 1. Product Runtime Authority Layer

This is the authoritative state layer inside Handshake.

It owns:

- workflow execution truth
- governed action truth
- capability and approval truth
- checkpoint and resume truth
- imported overlay binding truth

Primary runtime primitives already exist in product framing:

- `WorkflowRun`
- `WorkflowNodeExecution`
- `ModelSession`

The software-delivery overlay should extend those primitives with overlay-specific records rather than replacing them.

### 2. Overlay Definition Layer

This layer stores imported software-delivery governance artifacts as typed registry entries.

It owns:

- codex and protocol descriptors
- rubric descriptors
- check descriptors
- optional script descriptors as imported metadata only, not as default executable authority
- template descriptors
- schema descriptors
- version, provenance, and compatibility metadata

This layer defines what the overlay can ask for.
It does not define what is currently true for a running work item.

Current spec status:

- Governance Pack and Check Runner provide the hard constraints for this layer
- a standalone overlay registry record family is still architecture direction and draft-packet shape, not a separately named main-body authority section yet

### 3. Governed Execution Layer

This layer executes imported overlay actions through product-native bounded tools and workflows.

It owns:

- imported check execution
- imported rubric evaluation
- side-effecting overlay actions such as export, sync, merge, or closeout support
- explicit capability-gated and approval-gated execution

This layer must route through product tool gating, workflow state, and recorder visibility.

Current bounded-execution rule:

- the master spec currently hard-defines `governance.check.run` over imported registry entries of kind `Checks` or `Rubrics`
- arbitrary repo scripts are not implicitly executable authority; they should fail closed or be modeled separately before execution

### 4. Projection Layer

This layer turns authoritative runtime records into operator-facing views.

It owns:

- Dev Command Center views
- Task Board views
- Role Mailbox views
- compact summaries
- readable Markdown or export mirrors where useful

This layer never becomes execution authority.

### 5. Evidence Layer

This layer makes the control plane auditable and replayable.

It owns:

- Flight Recorder events
- typed run and node telemetry
- evidence artifact manifests
- replay bundles
- export records

## Canonical Runtime Objects

The target architecture should define these authoritative objects.

### 1. `GovernanceOverlayRegistryRecord`

Purpose:

- typed registry record for one imported software-delivery governance artifact

Minimum fields:

- `overlay_id`
- `overlay_version`
- `artifact_kind`
- `artifact_key`
- `source_snapshot`
- `source_hash`
- `project_profile_scope`
- `compatibility_status`
- `payload_ref`

Use:

- imported codex, protocols, rubrics, checks, templates, schemas

Authority:

- authoritative for overlay definition only
- this object family is the architecture target for imported overlay definition, but its exact standalone registry shape is still less codified than the Governance Pack and Check Runner rules it derives from

### 2. `GovernanceOverlayBinding`

Purpose:

- binds a project or workspace to one or more imported overlay versions

Minimum fields:

- `binding_id`
- `project_id`
- `overlay_id`
- `overlay_version`
- `activation_mode`
- `bound_profiles`
- `enabled_capabilities`
- `disabled_capabilities`
- `binding_status`

Use:

- determines which software-delivery governance overlay is active for a workspace or project kernel

Authority:

- authoritative for which overlay definitions are active

### 3. `GovernanceWorkContractRecord`

Purpose:

- authoritative structured execution contract for one software-delivery work item

This is the runtime replacement for treating the packet markdown as both contract and mutable ledger.

Minimum fields:

- `work_contract_id`
- `external_work_key`
- `contract_type`
- `project_id`
- `overlay_binding_id`
- `workflow_family`
- `scope_refs`
- `done_means`
- `risk_tier`
- `authority_assignment`
- `contract_status`
- `summary_ref`
- `source_artifact_refs`

Use:

- authoritative software-delivery contract record inside Handshake runtime

Authority:

- authoritative for bounded work contract meaning

### 4. `GovernanceWorkflowBindingRecord`

Purpose:

- binds a `GovernanceWorkContractRecord` to product workflow runtime

Minimum fields:

- `binding_id`
- `work_contract_id`
- `workflow_run_id`
- `active_node_ids`
- `queue_reason`
- `next_allowed_actions`
- `current_claims`
- `linked_gate_record_ids`
- `mirror_state`

Use:

- connects software-delivery work to the product workflow engine

Authority:

- authoritative runtime routing and progress binding

### 5. `GovernedActionRequest`

Purpose:

- typed request for side effects or decisions that cross capability, policy, or approval boundaries

Borrow direction:

- `PydanticAI` deferred request split
- `Letta` durable approval stop
- `BeeAI` inline permission requirement

Minimum fields:

- `action_request_id`
- `workflow_run_id`
- `workflow_node_execution_id`
- `work_contract_id`
- `action_kind`
- `requested_by_actor`
- `target_tool_or_runner`
- `payload_ref`
- `policy_basis`
- `requires_approval`
- `request_status`

Authority:

- authoritative pending governed action object

### 6. `GovernedActionResolution`

Purpose:

- durable resolution object for a governed action request

Minimum fields:

- `action_resolution_id`
- `action_request_id`
- `resolution_kind`
- `resolved_by_actor`
- `resolution_reason`
- `result_ref`
- `applied_capability_snapshot`
- `timestamp`

Resolution kinds should include at least:

- `approved`
- `denied`
- `executed`
- `retry_requested`
- `skipped`
- `unsupported`

Authority:

- authoritative result for a governed action request

### 7. `GovernanceValidatorGateRecord`

Purpose:

- authoritative validator gate state and verdict lineage for one software-delivery work contract or workflow-bound validation target

Minimum fields:

- `gate_record_id`
- `work_contract_id`
- `workflow_run_id`
- `workflow_node_execution_id`
- `gate_scope`
- `gate_phase`
- `gate_status_compat`
- `check_result_rollup`
- `validation_session_refs`
- `executed_check_result_refs`
- `latest_report_ref`

Use:

- keeps validator and check-runner posture queryable without overloading workflow binding or packet mirrors

Authority:

- authoritative gate posture, non-pass lineage, and validation result rollup for linked work

### 8. `GovernanceCheckpointRecord`

Purpose:

- authoritative checkpoint lineage for workflow-bound software-delivery state

Borrow direction:

- `CrewAI` checkpoint lineage
- `LangGraph` checkpoint metadata and lineage
- `Semantic Kernel` restore and migration discipline

Minimum fields:

- `checkpoint_id`
- `parent_checkpoint_id`
- `workflow_run_id`
- `work_contract_id`
- `checkpoint_kind`
- `snapshot_ref`
- `created_by`
- `resume_status`
- `fork_label`

Authority:

- authoritative resume and branch lineage object

### 9. `GovernanceEvidenceRecord`

Purpose:

- structured evidence bundle reference for one important workflow event, action, or closeout claim

Minimum fields:

- `evidence_id`
- `work_contract_id`
- `workflow_run_id`
- `node_execution_id`
- `evidence_kind`
- `artifact_refs`
- `event_refs`
- `summary_ref`
- `provenance_status`

Authority:

- authoritative evidence reference, not workflow truth by itself

## Authority Model

### What is authoritative

- imported overlay registry for overlay definitions
- overlay binding records for what overlay is active
- governance work contract records for software-delivery work meaning
- workflow bindings, governed action records, checkpoint records, and workflow runtime records for live state
- capability system for permission and denial semantics

### What is derived

- DCC views
- Task Board rows and lane layouts
- Role Mailbox summary views
- readable Markdown mirrors
- current status dashboards
- most human-readable closeout summaries

### What is evidence but not authority

- raw conversation logs
- thread chronology
- receipt streams
- session output tails
- exported patches alone
- legacy dossier narrative

## Execution Identity Model

Current repo governance spreads identity across packet ids, worktrees, branch names, session ids, and thread folders.
The target architecture should shrink that.

### Required identity split

- one stable external work key
  - example: `WP-1-Calendar-Storage-v2`
- one canonical runtime work contract id
- one workflow run id
- one or more workflow node execution ids
- one or more model session ids
- optional claim or lease ids for temporary ownership

### Design rule

Human-readable ids remain useful, but runtime joins must resolve through canonical ids rather than path conventions or thread chronology.

## Approval Model

Approval should stop being mostly implicit lane choreography.

The product-native approval model should be:

- typed request object
- typed resolution object
- capability snapshot captured at request and resolution time
- explicit target action identity
- recorder-visible stop and resume
- resumable without transcript reconstruction

Typical approval-bound action kinds:

- merge or branch-affecting actions
- imported check execution with elevated filesystem or tool scope
- governance pack export
- transcription of advisory mailbox or summary state into authoritative work state
- destructive cleanup or state override

## Projection Model

The projection rule should be strict.

### Dev Command Center

DCC is the canonical projection and control surface for:

- work contract state
- workflow run and node execution state
- validator gate state
- pending approvals
- route and claim state
- checkpoint lineage
- evidence and replay surfaces

### Task Board

Task Board is a planning projection over authoritative work contract and workflow state.
It must not become the authority for status, lane legality, or closeout truth.

### Role Mailbox

Role Mailbox is a collaboration and routing projection.
If a mailbox message changes work meaning, that meaning must be transcribed into authoritative records.

### Readable Markdown

Readable Markdown mirrors remain useful for export, review, and human scanning.
They should be treated as generated or synchronized views over structured records.
Projection-side edits should surface reconciliation posture explicitly rather than silently becoming authority.

## Evidence Model

The evidence plane should become simpler and stronger than the current dossier-heavy kernel.

### Mandatory recorder seams

- overlay binding activation
- work contract activation and supersession
- governed action request creation
- approval and denial results
- imported check runner lifecycle
- checkpoint create, resume, and fork
- handoff transcription into authoritative state
- terminal closeout outcome

### Evidence packaging

Evidence records should reference:

- Flight Recorder event ids
- artifact handles
- compact summaries
- optional replay bundles

The default operator path should be summary-first with drilldown, not raw-log-first archaeology.

## Imported Overlay vs Live Runtime State

This split is the most important architectural boundary.

### Imported overlay definition

Examples:

- codex
- role protocols
- rubrics
- check manifests
- templates
- schemas

These are:

- imported
- versioned
- typed
- provenance-linked

They are not live workflow truth.
They also do not automatically become executable authority just because they exist in repo `/.GOV/**`.

### Live runtime state

Examples:

- current work contract state
- current workflow state
- current claims and routing
- pending approvals
- checkpoint lineage
- current closeout posture

These are product-owned authoritative records.
They should live in product-owned runtime storage and not in repo `/.GOV/**`.

## Mapping From Current Repo-Governance Artifacts

| Current artifact | Future role inside Handshake | Classification |
| --- | --- | --- |
| `/.GOV/codex/...`, role protocols, rubrics, templates, schemas | imported overlay registry entries | imported overlay data |
| task packet markdown | source artifact plus readable contract mirror generated from `GovernanceWorkContractRecord` | imported source + projection |
| `TASK_BOARD.md` | Task Board projection over authoritative work contract/workflow state | projection |
| `WP_TRACEABILITY_REGISTRY.md` | compact summary or lookup projection over authoritative contract lineage | projection |
| `THREAD.md` | readable collaboration mirror; may inform evidence but not authority | projection/evidence |
| `RECEIPTS.jsonl` | event ingress and evidence stream; important but not canonical workflow truth | evidence |
| `RUNTIME_STATUS.json` | compatibility projection over authoritative runtime state | projection |
| `ROLE_SESSION_REGISTRY.json` | session/state projection backed by runtime session objects | projection |
| `SESSION_CONTROL_REQUESTS.jsonl` / `SESSION_CONTROL_RESULTS.jsonl` | command journal and audit trail for session control | evidence |
| validator gate files | `GovernanceValidatorGateRecord` family with compatibility import for legacy per-WP sessions and rollup into bounded check results | authoritative runtime state after migration |
| live workflow dossier | generated evidence summary over authoritative state and recorder events | projection/evidence |

## Migration Direction

### Preserve

- packet-style bounded contract semantics
- validator layering and final authority split
- non-pass states and evidence discipline
- manual relay fallback

### Adapt

- packets into structured work contract records plus readable mirrors
- validator gate files into authoritative `GovernanceValidatorGateRecord` runtime records
- session-control ledgers into recorder-visible session control journal
- receipt and mailbox traffic into typed handoff and action events with explicit transcription

### Drop

- packet as mutable operational ledger
- equal-truth competition between packet, board, runtime status, and gates
- repeated command-surface rediscovery
- closeout as a mostly manual reconciliation ceremony

## Phased Target

### Phase 1: Canonical software-delivery runtime records

- overlay registry
- overlay binding
- work contract record
- validator gate record
- governed action request and resolution
- basic workflow binding

### Phase 2: Product-native execution and approvals

- imported check runner
- capability-bound execution
- approval inbox and resume path in DCC
- recorder-visible execution and denial outcomes

### Phase 3: Projection and replay hardening

- compact summaries
- DCC route and checkpoint views
- replay bundles
- compatibility projections for readable contract and task board views

### Phase 4: Governance Pack instantiation

- export/import of software-delivery overlay definitions
- profile-specific instantiation for non-Handshake projects
- conformance harness for equivalent gate semantics

## Working Recommendation

The target architecture should be built around one sentence:

- imported software-delivery governance defines the overlay, but Handshake runtime owns the truth

That means the implementation sequence should favor:

1. canonical runtime objects
2. governed action and approval model
3. imported check runner
4. projection surfaces
5. governance pack instantiation

Not the reverse.

## Immediate Follow-On Questions

- which exact fields from current task packets belong in `GovernanceWorkContractRecord` versus compact summaries
- how much of current receipt taxonomy can be normalized directly into handoff/action event types
- how much current session-control journaling should remain queryable as first-class evidence after runtime-native session control exists
- whether validator gate lifecycle should remain one record family with `gate_scope` and `gate_phase`, or later split into gate summary plus per-check execution children

Current status:

- the first-pass field placement and primitive sketch now exists in `Handshake_Governance_Runtime_Schema_Sketch_WORKING.md`
- the registry and bounded execution claims are now narrowed to what the master spec clearly supports today: Governance Pack constraints, additive imported overlay posture, and `governance.check.run` over `Checks` and `Rubrics`
