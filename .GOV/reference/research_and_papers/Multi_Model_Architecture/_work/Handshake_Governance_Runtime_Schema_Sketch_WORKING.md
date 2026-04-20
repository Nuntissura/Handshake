# Handshake Governance Runtime Schema Sketch Working Note

Temporary working note that refines the target architecture into concrete schema and primitive candidates.

This is not implementation code.
It is the field-level sketch that should eventually feed:

- primitive definitions
- schema registry entries
- target packets for product implementation

## Purpose

- turn the target architecture into concrete record shapes
- align those shapes with existing Handshake base-envelope and workflow-state law
- clarify which current repo-governance fields belong in canonical detail, compact summaries, projections, or imported overlay data

## Governing Constraints

### 1. Shared base envelope

For canonical structured collaboration artifacts, the existing base envelope must remain the starting point:

- `schema_id`
- `schema_version`
- `record_id`
- `record_kind`
- `project_profile_kind`
- `updated_at`
- `mirror_state`
- `authority_refs`
- `evidence_refs`

Source:

- `Handshake_Master_Spec_v02.180.md` lines `6843-6852`

Implication:

- software-delivery overlay records that participate in canonical collaboration flows should remain field-equivalent at the base-envelope level with the existing project-agnostic collaboration record family
- that equivalence matters across canonical detail, compact summaries, Task Board projections, and Role Mailbox exports, even when the software-delivery overlay adds profile-specific fields

### 2. Compact summary contract

When a record participates in operator routing, small-model planning, or inbox/board rendering, it should have a paired compact summary preserving:

- `record_id`
- `record_kind`
- `project_profile_kind`
- `status`
- `title_or_objective`
- `blockers`
- `next_action`
- `authority_refs`
- `evidence_refs`
- `updated_at`

Source:

- `Handshake_Master_Spec_v02.180.md` lines `6866-6878`

Implication:

- compact summary should be the default first-read path for DCC, Task Board derived layouts, Role Mailbox triage, and local-small-model planning
- canonical detail and compact summary should share deterministic join fields, especially `record_id`, `project_profile_kind`, `authority_refs`, and `evidence_refs`

### 3. Workflow-state triplet

Any queueable or operational software-delivery record should expose the workflow-state triplet:

- `workflow_state_family`
- `queue_reason_code`
- `allowed_action_ids`

Those action ids must resolve to registered governed-action descriptors rather than ad hoc UI verbs.

Projection order, mailbox chronology, or board lane position must never substitute for `workflow_state_family` or `queue_reason_code`.

Local-small-model eligibility should be treated as an explicit policy outcome over this state, not as a UI hint.
The starting assumption should stay aligned with current spec law:

- only records in a ready family that are explicitly `ready_for_local_small_model` should auto-qualify
- approval-bound, protected, or semantic-rewrite actions should stay excluded unless capability policy explicitly grants them

Source:

- `Handshake_Master_Spec_v02.180.md` lines `6950-6993`
- `WP-1-Project-Agnostic-Workflow-State-Registry-v1/packet.md` lines `348-357`

### 4. Profile extension boundary

Repo-specific or software-delivery-specific details such as branch names, worktree paths, CI gate identifiers, or coding-language hints should live in `profile_extension`, not in universal base fields.

### 5. Projection boundary

DCC, Task Board, and Role Mailbox are projection and control surfaces, not alternate sources of authority.

That implies:

- authoritative execution changes must resolve through canonical JSON or JSONL records plus governed actions
- projections should expose visible reconciliation state such as `authority_mode`, `reconciliation_action`, and `manual_edit_zones` where the surface permits edits
- mailbox thread lifecycle, claim or lease handling, and announce-back or handoff messaging must not silently mutate authoritative work state without governed transcription

## Baseline Workflow-State And Governed-Action Taxonomy

The software-delivery overlay should not invent a separate routing language.
It should adopt the project-agnostic workflow-state and governed-action contract already defined in the master spec, then narrow or relabel it through project-profile extensions only where allowed.

### Phase 1 base workflow-state families

Adopt these families as the canonical routing surface for software-delivery work:

- `intake`
- `ready`
- `active`
- `waiting`
- `review`
- `approval`
- `validation`
- `blocked`
- `done`
- `canceled`
- `archived`

Interpretation should stay aligned with the master spec:

- `intake`: known work awaiting triage or decomposition
- `ready`: executable work with enough context, dependencies, and permissions to begin
- `active`: work currently being executed by a human, model, or workflow engine
- `waiting`: work expected to resume after an external response, dependency clear, or retry window
- `review`: work awaiting review rather than fresh execution
- `approval`: work awaiting an explicit governance or operator decision
- `validation`: work awaiting deterministic checks, rubric checks, or acceptance verification
- `blocked`: work that cannot safely progress until a blocker is cleared
- `done`: completed work that remains visible to active operating views
- `canceled`: explicitly stopped work not expected to resume automatically
- `archived`: closed work retained for history, evidence, or search only

### Phase 1 base queue reasons

Reuse the spec vocabulary directly instead of creating packet-local or lane-local reasons:

- `needs_triage`
- `dependency_wait`
- `mailbox_response_wait`
- `mailbox_snoozed`
- `human_review_wait`
- `decision_wait`
- `approval_wait`
- `validation_wait`
- `escalation_wait`
- `mailbox_expired`
- `dead_letter_remediation`
- `operator_pause`
- `policy_block`
- `resource_unavailable`
- `retry_scheduled`
- `ready_for_local_small_model`
- `ready_for_cloud_model`
- `completed`
- `rejected`
- `canceled`

Overlay guidance:

- work contracts and workflow bindings may use the full family and reason set
- governed action requests should normally narrow to `approval_wait`, `decision_wait`, `retry_scheduled`, `completed`, `rejected`, or `canceled`
- validator gate records should normally use `validation_wait`, `completed`, `policy_block`, or `resource_unavailable`
- board lanes, mailbox order, and readable packet labels may relabel the surface, but must not fork these base meanings

### Starter governed-action descriptor set

The software-delivery overlay should begin with one stable governed-action starter set mapped onto the existing `GovernedActionDescriptorV1` verb contract.
Recommended starter ids:

- `gov.triage`
- `gov.start`
- `gov.delegate`
- `gov.request_review`
- `gov.request_decision`
- `gov.escalate`
- `gov.reply`
- `gov.retry`
- `gov.reroute`
- `gov.validate`
- `gov.approve`
- `gov.reject`
- `gov.complete`
- `gov.cancel`
- `gov.archive`

Working interpretation:

- `gov.triage`: intake shaping and decomposition
- `gov.start`: move executable work into active execution
- `gov.delegate`: hand work to another executor or workflow path without losing contract identity
- `gov.request_review`: place work into explicit review posture
- `gov.request_decision`: place work into operator or governance decision posture
- `gov.escalate`: move work toward a higher-authority or higher-capability actor
- `gov.reply`: satisfy a mailbox-linked or reviewer-linked response obligation
- `gov.retry`: resume blocked, waiting, or validation-constrained work after a bounded retry condition
- `gov.reroute`: change queue placement or responsible execution path without mutating the contract itself
- `gov.validate`: invoke deterministic or rubric-backed validation posture
- `gov.approve`: resolve an approval boundary positively
- `gov.reject`: resolve an approval, review, or validation boundary negatively
- `gov.complete`: close bounded work after execution and any required validation
- `gov.cancel`: stop work without archive semantics
- `gov.archive`: retain closed work for history only

These ids should remain stable even when the visible UI labels differ by project profile.
Exact `allowed_from_families`, `result_family`, `result_reason_code`, actor allowlists, and evidence requirements should be encoded in `GovernedActionDescriptorV1`, `WorkflowTransitionRuleV1`, and `ExecutorEligibilityPolicyV1`, not re-inferred from prose.

## Primitive Inventory

| Candidate primitive | Kind | Role |
| --- | --- | --- |
| `PRIM-GovernanceOverlayRegistryRecordV1` | `rust_struct` + `spec_schema` | imported overlay artifact definition |
| `PRIM-GovernanceOverlayBindingV1` | `rust_struct` + `spec_schema` | active overlay binding for a project/workspace |
| `PRIM-GovernanceWorkContractV1` | `rust_struct` + `spec_schema` | canonical software-delivery work contract |
| `PRIM-GovernanceWorkContractSummaryV1` | `spec_schema` | compact summary for the work contract |
| `PRIM-GovernanceWorkflowBindingV1` | `rust_struct` + `spec_schema` | runtime binding between work contract and workflow execution |
| `PRIM-GovernedActionRequestV1` | `rust_struct` + `spec_schema` | approval/deferred-side-effect request |
| `PRIM-GovernedActionResolutionV1` | `rust_struct` + `spec_schema` | durable resolution for a governed action |
| `PRIM-GovernanceValidatorGateRecordV1` | `rust_struct` + `spec_schema` | authoritative validator gate state and verdict lineage |
| `PRIM-GovernanceCheckpointRecordV1` | `rust_struct` + `spec_schema` | checkpoint lineage and resume object |
| `PRIM-GovernanceEvidenceRecordV1` | `rust_struct` + `spec_schema` | durable evidence/artifact reference record |
| `PRIM-SoftwareDeliveryProfileExtensionV1` | `spec_schema` | repo/worktree/branch/check-specific extension fields |

## Record Family Rules

### Pure registry records

These are not queueable work items.
They should use the base envelope but do not need the workflow-state triplet by default.

Applies to:

- overlay registry records
- some evidence manifests

### Queueable operational records

These are live workflow or approval objects.
They should use:

- base envelope
- workflow-state triplet
- optional compact summary

Applies to:

- work contracts
- workflow bindings
- governed action requests
- validator gate records

### Terminal or historical records

These should keep the base envelope and strong authority/evidence references, but may not need a queue-centric summary after closure.

Applies to:

- action resolutions
- checkpoints
- evidence records

## Proposed Schemas

### 1. `PRIM-GovernanceOverlayRegistryRecordV1`

Kind:

- `rust_struct`
- `spec_schema`

Record kind:

- `governance_overlay_registry_record`

Schema sketch:

```text
base_envelope:
  schema_id: PRIM-GovernanceOverlayRegistryRecordV1
  schema_version: 1
  record_id
  record_kind
  project_profile_kind
  updated_at
  mirror_state
  authority_refs
  evidence_refs

overlay_fields:
  overlay_id
  overlay_version
  artifact_kind
  artifact_key
  source_snapshot
  source_hash
  payload_ref
  compatibility_status
  import_mode
  source_authority_kind
  profile_extension?: SoftwareDeliveryProfileExtensionV1
```

Notes:

- authoritative for imported overlay definition only
- this is an architecture-direction record family inferred from Governance Pack and check-runner constraints plus draft overlay packets; it should not be presented as a fully codified standalone main-body law yet
- no workflow-state triplet by default

### 2. `PRIM-GovernanceOverlayBindingV1`

Kind:

- `rust_struct`
- `spec_schema`

Record kind:

- `governance_overlay_binding`

Schema sketch:

```text
base_envelope:
  shared fields

binding_fields:
  binding_id
  project_id
  workspace_id
  overlay_id
  overlay_version
  activation_mode
  binding_status
  bound_profiles
  enabled_capabilities
  disabled_capabilities
  imported_artifact_refs
  profile_extension?: SoftwareDeliveryProfileExtensionV1
```

Notes:

- may expose a compact summary for inspection, but is not itself the work queue object

### 3. `PRIM-GovernanceWorkContractV1`

Kind:

- `rust_struct`
- `spec_schema`

Record kind:

- `governance_work_contract`

Schema sketch:

```text
base_envelope:
  shared fields

workflow_state:
  workflow_state_family
  queue_reason_code
  allowed_action_ids

contract_fields:
  work_contract_id
  external_work_key
  contract_type
  project_id
  overlay_binding_id
  title
  objective
  contract_status
  risk_tier
  workflow_family
  scope_refs
  done_means
  authority_assignment
  source_artifact_refs
  summary_ref
  active_binding_ref?
  supersedes_record_id?
  profile_extension?: SoftwareDeliveryProfileExtensionV1
```

Required `authority_assignment` shape:

```text
workflow_authority
technical_advisor?
technical_authority
merge_authority?
operator_authority?
```

Notes:

- this is the canonical replacement for using packet markdown as both contract and mutable ledger
- packet markdown becomes a source artifact plus readable mirror over this record

### 4. `PRIM-GovernanceWorkContractSummaryV1`

Kind:

- `spec_schema`

Record kind:

- `governance_work_contract_summary`

Schema sketch:

```text
summary_contract:
  record_id
  record_kind
  project_profile_kind
  status
  title_or_objective
  blockers
  next_action
  authority_refs
  evidence_refs
  updated_at

workflow_state:
  workflow_state_family
  queue_reason_code
  allowed_action_ids
```

Notes:

- should be the default DCC/task-board/Role-Mailbox/local-small-model load path
- should remain join-compatible with canonical detail through `record_id`, `project_profile_kind`, `authority_refs`, and `evidence_refs`

### 5. `PRIM-GovernanceWorkflowBindingV1`

Kind:

- `rust_struct`
- `spec_schema`

Record kind:

- `governance_workflow_binding`

Schema sketch:

```text
base_envelope:
  shared fields

workflow_state:
  workflow_state_family
  queue_reason_code
  allowed_action_ids

binding_fields:
  binding_id
  work_contract_id
  workflow_run_id
  active_node_execution_ids
  current_claim_ids
  current_session_ids
  next_expected_actor
  transition_rule_ids
  executor_eligibility_policy_ids
  automation_rule_ids
  mirror_state_reason?
```

Notes:

- binds the contract to live runtime and projection law
- this is the main bridge record DCC should query

### 6. `PRIM-GovernedActionRequestV1`

Kind:

- `rust_struct`
- `spec_schema`

Record kind:

- `governed_action_request`

Schema sketch:

```text
base_envelope:
  shared fields

workflow_state:
  workflow_state_family
  queue_reason_code
  allowed_action_ids

request_fields:
  action_request_id
  work_contract_id
  workflow_run_id
  workflow_node_execution_id
  requested_by_actor
  action_descriptor_id
  target_tool_or_runner
  payload_ref
  policy_basis
  capability_snapshot_ref
  approval_required
  request_status
  supersedes_request_id?
  profile_extension?: SoftwareDeliveryProfileExtensionV1
```

Notes:

- `allowed_action_ids` should normally narrow to actions relevant to resolving the request
- this is the typed stop object for approval or deferred side effects

### 7. `PRIM-GovernedActionResolutionV1`

Kind:

- `rust_struct`
- `spec_schema`

Record kind:

- `governed_action_resolution`

Schema sketch:

```text
base_envelope:
  shared fields

resolution_fields:
  action_resolution_id
  action_request_id
  resolution_kind
  resolved_by_actor
  resolution_reason
  result_ref
  capability_snapshot_ref
  produced_event_refs
  produced_artifact_refs
```

Resolution kinds:

- `approved`
- `denied`
- `executed`
- `retry_requested`
- `skipped`
- `unsupported`

Notes:

- usually terminal or history-oriented; no queue summary required by default

### 8. `PRIM-GovernanceValidatorGateRecordV1`

Kind:

- `rust_struct`
- `spec_schema`

Record kind:

- `governance_validator_gate_record`

Schema sketch:

```text
base_envelope:
  shared fields

workflow_state:
  workflow_state_family
  queue_reason_code
  allowed_action_ids

gate_fields:
  gate_record_id
  work_contract_id
  workflow_run_id
  workflow_node_execution_id?
  gate_scope
  gate_target_ref
  gate_phase
  gate_status_compat
  check_result_rollup
  active_validation_session_id?
  validation_session_refs
  archived_session_refs
  required_check_descriptor_ids
  executed_check_result_refs
  last_transition_event_ref?
  latest_report_ref?
  legacy_import_ref?
  profile_extension?: SoftwareDeliveryProfileExtensionV1
```

Required taxonomy:

- `gate_scope`: `pre_work` | `post_work` | `custom`
- `gate_phase`: `pending` | `presented` | `acknowledged` | `appending` | `committable` | `committed` | `archived`
- `gate_status_compat`: `pending` | `pass` | `fail` | `skip`
- `check_result_rollup`: `PASS` | `FAIL` | `BLOCKED` | `ADVISORY_ONLY` | `UNSUPPORTED`

Working state guidance:

- active or pending gate work should usually surface as `workflow_state_family=validation`, `queue_reason_code=validation_wait`
- successful or advisory completion should usually surface as `workflow_state_family=done`, `queue_reason_code=completed`
- failed validation that blocks linked work should usually surface as `workflow_state_family=blocked`, `queue_reason_code=validation_wait`
- capability-denied or unsupported gate execution should usually surface as `workflow_state_family=blocked` with `queue_reason_code=policy_block` or `resource_unavailable`, depending on cause

Notes:

- this should be a dedicated record family, not just a `GovernanceWorkflowBindingV1` extension
- the reason is that gate state has its own lifecycle, verdict vocabulary, evidence lineage, and legacy per-WP import shape
- legacy `pending` / `pass` / `fail` / `skip` compatibility should remain explicit while newer bounded check execution uses the `CheckResult` rollup

### 9. `PRIM-GovernanceCheckpointRecordV1`

Kind:

- `rust_struct`
- `spec_schema`

Record kind:

- `governance_checkpoint_record`

Schema sketch:

```text
base_envelope:
  shared fields

checkpoint_fields:
  checkpoint_id
  parent_checkpoint_id?
  work_contract_id
  workflow_run_id
  checkpoint_kind
  snapshot_ref
  created_by
  resume_status
  fork_label?
  restoration_compatibility
```

Notes:

- should be directly usable by DCC recovery and replay views

### 10. `PRIM-GovernanceEvidenceRecordV1`

Kind:

- `rust_struct`
- `spec_schema`

Record kind:

- `governance_evidence_record`

Schema sketch:

```text
base_envelope:
  shared fields

evidence_fields:
  evidence_id
  work_contract_id?
  workflow_run_id?
  workflow_node_execution_id?
  evidence_kind
  artifact_refs
  event_refs
  summary_ref?
  provenance_status
```

Notes:

- evidence record is authoritative about evidence references, not about workflow truth

### 11. `PRIM-SoftwareDeliveryProfileExtensionV1`

Kind:

- `spec_schema`

Purpose:

- isolates software-delivery-specific fields that should not pollute the universal base envelope

Extension field groups:

```text
repo_identity:
  repository_id?
  remote_url?
  default_branch?

worktree_binding:
  worktree_ref?
  branch_ref?
  merge_base_ref?

software_delivery_policy:
  required_check_ids?
  check_runner_profile?
  merge_policy?
  review_policy?

source_import:
  imported_artifact_path?
  imported_layout_version?
```

Notes:

- keep this optional
- do not require every Handshake kernel to understand these fields

## Proposed Action Registry Dependency

The architecture note already depends on governed action request and resolution objects.
This schema sketch makes one extra dependency explicit:

- `allowed_action_ids` must resolve to registered `GovernedActionDescriptorV1` records

That means the software-delivery overlay needs to consume, not bypass, the existing project-agnostic governed-action registry direction.

## Current Packet Field Mapping

This is the first-pass answer to which current packet fields should land where.

### Keep in canonical work contract detail

- stable external work key
- risk tier
- authority assignment
- bounded scope references
- done means
- overlay binding
- source artifact references
- supersession lineage

### Move to workflow binding or runtime execution records

- next actor
- active node/session ids
- current claims
- transition/action legality
- queue reason
- approval-wait posture
- closeout readiness

### Keep in dedicated validator gate records

- current pre-work and post-work gate posture
- gate session lineage
- executed check result refs
- compatibility verdict mapping for legacy gate state
- non-pass rationale and supporting evidence refs
- gate transition chronology that should remain queryable without dossier archaeology

### Keep as imported overlay definition, not live runtime truth

- codex identity
- protocol descriptors
- rubric descriptors
- check descriptors
- script descriptors as imported metadata only unless and until they are separately modeled under bounded executable authority
- template descriptors
- source snapshot hashes

### Keep only in readable mirrors or evidence

- verbose narrative packet prose
- raw handoff narrative
- thread chronology
- receipt chronology
- long workflow dossier narrative

### Likely drop as hand-maintained runtime truth

- packet fields that duplicate live session registry state
- packet fields that duplicate runtime status
- packet fields that exist only to reconcile stale task-board state
- packet-local command routing hints that should become action-descriptor or policy data

## Candidate Summary/View Defaults

### DCC default

Load order should be:

1. `GovernanceWorkContractSummaryV1`
2. `GovernanceWorkflowBindingV1`
3. `GovernanceValidatorGateRecordV1` when validation or gate posture affects routing
4. `GovernedActionRequestV1` if waiting on approval or action
5. canonical detail record only on drilldown

DCC should also surface:

- `authority_mode`
- `reconciliation_action`
- `manual_edit_zones`
- checkpoint lineage when recovery is possible

### Task Board default

Task Board rows should derive from:

- `GovernanceWorkContractSummaryV1`
- `GovernanceWorkflowBindingV1`
- summarized gate posture projected from `GovernanceValidatorGateRecordV1` when validation blocks or completes work

not from packet-local prose or mailbox chronology.

Board order or lane placement must never stand in for `workflow_state_family`, `queue_reason_code`, or `allowed_action_ids`.

### Role Mailbox default

Mailbox threads should link to:

- `record_id`
- `workflow_state_family`
- `queue_reason_code`
- `allowed_action_ids`
- authoritative transcription status

Mailbox should keep collaboration chronology separate from authority state.
In particular:

- claim, lease, and takeover signals should link to authoritative state rather than becoming authority by themselves
- announce-back, handoff, or summary replies should require explicit governed transcription before they change canonical work meaning

## Primitive Follow-On Set

If this sketch is accepted, the next packet/spec work should probably materialize:

1. `PRIM-GovernanceWorkContractV1`
2. `PRIM-GovernedActionRequestV1`
3. `PRIM-GovernedActionResolutionV1`
4. `PRIM-GovernanceWorkflowBindingV1`
5. `PRIM-GovernanceValidatorGateRecordV1`
6. `PRIM-GovernanceOverlayRegistryRecordV1`

Those six primitives would be enough to anchor:

- imported overlay registry
- product-native work contract truth
- approval/deferred-side-effect handling
- validator gate truth and non-pass lineage
- projection and routing

## Open Questions

- should `GovernanceWorkContractV1` directly embed compact-summary-critical fields, or should summary generation remain fully derived
- should `GovernanceOverlayBindingV1` be queueable when an overlay import/migration is pending, or remain pure registry/config state
- how much current packet metadata should survive as imported source artifact content after canonical contract migration
- whether a standalone overlay-registry primitive should wait for stronger main-body codification, or proceed as an implementation-side schema aligned to Governance Pack and Check Runner constraints
- whether gate lifecycle should stay one record family with `gate_scope` and `gate_phase`, or split later into a higher-level gate record plus per-check execution children if bounded validation volume grows
