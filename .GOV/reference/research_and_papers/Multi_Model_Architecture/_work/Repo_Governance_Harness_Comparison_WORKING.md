# Repo Governance Harness Comparison Working Note

Temporary working note to resume the current research line inside `Multi_Model_Architecture/`.
This document compares the current repo-governance testbed against external harness mechanisms and then translates the useful parts into a Handshake-native direction.

It is not a final architecture document.

## Purpose

- compare external harnesses against the actual repo-governance testbed, not against an idealized future state
- use repo-inspection evidence for the brittle baseline
- keep the real target clear: a better software-delivery governance overlay implemented inside Handshake using Handshake tools, features, and topology

## Framing Correction

Handshake is broader than a coding IDE or repo-governance shell.
It is a local-first creative and execution product with its own workflow engine, capability system, Command Center, Flight Recorder, and project-agnostic governance surfaces.

But that does not make current repo governance irrelevant.

The current repo-governance kernel is still valuable because:

- it is the live software-delivery overlay used while building Handshake
- it mirrors a real subset of the future Handshake control problem
- it exposes brittle control-plane behavior under real pressure
- it gives us failure-backed evidence about what a better repo-governance implementation inside Handshake must fix

So the right comparison is not:

- external harnesses versus Handshake product in the abstract

It is:

- current repo-governance testbed
- external harness mechanisms
- Handshake-native repo-governance overlay design

## Evidence Base

- repo-governance inspection and failure analysis:
  - `.GOV/reference/research_and_papers/Multi_Model_Architecture/Workflow_State_Packet_Truth_and_Range_Drift.md`
  - `.GOV/reference/research_and_papers/Multi_Model_Architecture/Repo_Governance_Failure_Taxonomy.md`
  - `.GOV/reference/research_and_papers/Multi_Model_Architecture/Kernel_to_Swarm_Gap_Map.md`
  - `.GOV/reference/research_and_papers/Multi_Model_Architecture/Harness_Lessons_Learned.md`
  - `.GOV/reference/REPO_GOVERNANCE_EXTERNAL_REVIEW_OVERVIEW.md`
- external harness extraction:
  - `.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Harness_Adoption_Extraction_WORKING.md`
  - `.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Typed_Runtime_Resume_Approval_WORKING.md`
- live repo-governance audit evidence:
  - `.GOV/Audits/smoketest/AUDIT_20260410_GOVERNANCE_WORKFLOW_MIRROR_ACTIVATION_MANAGER_SMOKETEST_REVIEW.md`
- product translation constraints:
  - `.GOV/spec/Handshake_Master_Spec_v02.180.md`
  - `.GOV/spec/HANDSHAKE_PRODUCT_REFERENCE.md`
  - `.GOV/task_packets/WP-1-Product-Governance-Snapshot-v4.md`

## Current Repo-Governance Read

The current repo-governance system is not failing because the authority model is weak in principle.
It is failing because too much workflow truth and too much repair logic are spread across too many surfaces.

The strongest current-kernel assets still look worth preserving:

- explicit workflow versus technical authority split
- bounded Work Packet contracts
- adversarial validator posture
- strong skepticism toward chat-only completion claims
- degraded manual relay as a real fallback mode

The most expensive brittle seams are:

1. workflow truth is fragmented across packet, task board, runtime status, receipts, session registry, control ledgers, and gate outputs
2. the packet acts as both signed contract and mutable operational ledger
3. session lifecycle convergence is still repair-heavy
4. handoff and next-actor routing still rely on too much interpretation
5. closeout convergence still needs too many surfaces to agree at once
6. too much operator or orchestrator skill is still required for normal recovery
7. token and time burn are amplified by inspection and repair churn rather than product work

## Comparison Matrix

| Repo-governance problem | Current repo-governance posture | Strong external mechanisms | Handshake-native repo-governance direction |
| --- | --- | --- | --- |
| Workflow truth authority | packet, task board, runtime status, receipts, registries, and gates all participate in truth; drift becomes a control-plane incident | `LangGraph` checkpoint lineage and explicit run state; `Semantic Kernel` save/load plus migration discipline; `AutoGen` versioned state envelopes | one authoritative workflow-state family under Handshake runtime, with packet/task board/mailbox as projections rather than competing truth islands |
| Session lifecycle convergence | ACP request/result ledgers and session registry can still disagree; settled state often needs repair | `Open SWE` reconnect-by-thread and deterministic execution identity; `CrewAI` checkpoint lineage and resume; `LangGraph` thread lifecycle APIs | one durable session or run object bound to workflow identity, with explicit recover, resume, cancel, and settle transitions visible in product runtime and DCC |
| Approval and governed side effects | approval and authority often live in procedural choreography and lane rules more than in typed runtime objects | `PydanticAI` deferred request/result envelopes; `Letta` durable approval stops; `BeeAI` inline permission requirements; `Cline` semantic approval identity | governed action envelopes plus approval-result envelopes routed through Handshake capability gates, workflow state, and Flight Recorder |
| Handoff and work transfer | receipts and thread artifacts carry real signal, but next-actor truth can still lag or drift | `Open SWE` queued follow-up injection; `Roo Code` durable parent-child return path; `AutoGen` typed handoff events; `OpenAI Swarm` lower-bound handoff semantics | structured handoff bundles with explicit transcription into authoritative workflow records and mailbox projections, not thread chronology alone |
| Evidence and replay | dossiers, audits, and receipts are rich but upkeep-heavy and sometimes placeholder-prone | `SWE-agent` replay bundles; `TaskWeaver` event taxonomy and typed attachments; `Letta` split run/step telemetry; `OpenHands` temporary artifact hygiene | Flight Recorder-linked evidence artifacts, compact summaries, and typed run/step/event records instead of manual dossier-first upkeep |
| Tool and policy gating | command-surface drift and wrapper drift still create wrong-tool failures and repair loops | `BeeAI` requirement compiler; `smolagents` tool validation; `Roo Code` execution-time scope validation; `Cline` hook and approval policy | typed tool registry plus policy-as-data and bounded action families inside Handshake, with mechanical rejection before runtime drift spreads |
| Parallel coordination | current kernel supports narrow governed parallelism but not cheap high-concurrency ownership transfer | `LangGraph` `Command` and `Send`; `AgentScope` `MsgHub` and state backends; `PocketFlow` explicit action-routed control flow | explicit claim, lease, ownership, and fan-out objects tied to workflow state and worktree or resource bindings, not inferred from thread order |
| Degraded manual fallback | manual relay exists and is useful, but it still feels like a different operational mode | `Open SWE` busy-run queueing; `LangGraph` interrupt/resume; `CrewAI` restore/fork semantics | keep one runtime state model for both autonomous orchestration and manual relay, with different control surfaces over the same authority records |

## What Should Carry Forward

### Preserve from current repo governance

- authority split between workflow coordination and final technical judgment
- explicit packet-like execution contracts for software-delivery work
- hard evidence requirement for validator closure
- bounded worktree and scope discipline
- willingness to preserve non-pass states instead of narrating success

### Borrow from external harnesses

- explicit canonical run state with lineage and resume semantics
- typed approval and deferred-side-effect envelopes
- deterministic execution identity
- explicit handoff objects and queued follow-ups
- split run versus step telemetry
- checkpoint lineage, replay, and compact operator restore surfaces
- policy-as-data rather than prompt-only governance

### Shrink or drop from current repo governance

- packet fields that behave like hand-maintained runtime ledgers
- repeated truth duplication across packet, board, runtime status, and gate outputs
- closeout as a repair-heavy reconciliation ritual
- command-surface rediscovery during ordinary operation
- thread or receipt chronology being treated as quasi-authoritative state
- manual dossier upkeep for information the runtime already knows

## Better Repo Governance Inside Handshake

The likely end state is not "import repo governance as-is into Handshake."
It is "rebuild the software-delivery governance overlay on top of Handshake-native runtime surfaces."

That better overlay should use:

- product-owned workflow state under `.handshake/gov/`
- product workflow primitives such as `WorkflowRun`, `WorkflowNodeExecution`, and `ModelSession`
- Dev Command Center for operator projection, route inspection, approvals, and recovery actions
- Flight Recorder for durable run, step, approval, and export evidence
- Role Mailbox and Task Board as readable projections over authoritative records
- Governance Pack, artifact registry, and check-runner surfaces for the software-delivery overlay
- Handshake tools and bounded execution surfaces instead of raw repo-governance shell choreography

## Working Conclusion

The research direction should remain:

- keep inspecting external harnesses for concrete mechanisms
- compare those mechanisms directly against repo-inspection evidence from the current testbed
- do not treat repo governance as disposable
- do not treat repo governance as the universal architecture either

Repo governance is the live software-delivery testbed.
Handshake is the broader product control plane.
The real design job is to build a stronger repo-governance overlay inside Handshake by preserving the current kernel's authority strengths while replacing its fragmented truth model, repair-heavy session control, and inspection-heavy runtime behavior.

## Immediate Follow-On

The next architecture document should answer this specific question:

- what are the canonical Handshake runtime objects for software-delivery governance, and which current repo-governance artifacts become authoritative records versus readable projections versus imported overlay data

That document should be written after this note, not before it.
