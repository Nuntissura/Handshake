# Handshake Governance Direction Working Note

Temporary working note that bridges the external harness extraction to the already-declared Handshake product-governance direction.
This is not a final architecture doc. It is a decision-orientation memo for the next research step.

## Purpose

- state clearly whether repo governance should remain the main build surface
- anchor that answer in current Handshake product-governance artifacts, not only in repo-governance lessons
- identify which extracted harness mechanisms map onto the Handshake target surface

## Evidence Base

- external harness extraction:
  - `.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Harness_Adoption_Extraction_WORKING.md`
- typed runtime follow-up on resume, deferred execution, and approvals:
  - `.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Typed_Runtime_Resume_Approval_WORKING.md`
- compare layer between current repo-governance testbed, external harnesses, and Handshake-native adoption:
  - `.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Repo_Governance_Harness_Comparison_WORKING.md`
- product-translation bridge between master-spec law, product surfaces, harness mechanisms, and repo-governance migration:
  - `.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Handshake_Governance_Product_Translation_WORKING.md`
- target architecture synthesis for the Handshake-native software-delivery governance overlay:
  - `.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Handshake_Governance_Target_Architecture_WORKING.md`
- current product-governance boundary and runtime state target:
  - `.GOV/task_packets/WP-1-Product-Governance-Snapshot-v4.md`
  - `.GOV/spec/Handshake_Master_Spec_v02.180.md`
- backlog stubs for product-owned governance import, execution, and projection:
  - `.GOV/task_packets/stubs/WP-1-Product-Governance-Artifact-Registry-v1.md`
  - `.GOV/task_packets/stubs/WP-1-Product-Governance-Check-Runner-v1.md`
  - `.GOV/task_packets/stubs/WP-1-Governance-Workflow-Mirror-v1.md`
  - `.GOV/task_packets/stubs/WP-1-Governance-Pack-v1.md`

## Current Conclusion

- repo governance should stop being treated as the main build destination
- repo governance should remain a reference corpus, failure testbed, and mechanism source
- Handshake governance should be designed as a product-owned runtime with a bounded imported software-delivery overlay
- the old embedded repo-governance copy inside the product should not be the seed architecture

That is not just a strategy preference. It is already the direction implied by the product-side artifacts.

## Clarified Framing

This does not mean repo governance is irrelevant or should be discarded.

- Handshake is broader than a software IDE or repo-governance shell
- repo governance is still the live software-delivery testbed used to build Handshake
- the comparison target for external harness research should therefore be:
  - current repo-governance failure evidence
  - external harness mechanisms
  - Handshake-native software-delivery governance overlay design

The important boundary is:

- do not keep growing the current repo kernel as if it were the final product architecture
- do keep mining it as the highest-value failure-rich software-delivery overlay and migration source

## Why This Conclusion Is Backed By The Repo

### 1. Product runtime governance already has a separate authority boundary

- `WP-1-Product-Governance-Snapshot-v4` explicitly moves runtime governance state to product-owned storage, default `.handshake/gov/`, and removes runtime dependencies on repo `docs/**` and `/.GOV/**`.
- `Handshake_Master_Spec_v02.180.md` repeats the same boundary in `7.5.4.8 Governance Pack` and the surrounding governance/runtime sections.

Implication:
- the product target is already "product-owned runtime governance state", not "live repo-governance files inside the app"

### 2. Imported repo governance is defined as an additive overlay, not universal authority

- `WP-1-Product-Governance-Artifact-Registry-v1` says imported software-delivery governance must be versioned, typed, provenance-linked, and additive
- it explicitly says imported overlay must not replace Handshake-native governance or broader project-profile rules

Implication:
- Handshake governance is broader than repo governance
- the current repo-governance surface is one future overlay, not the whole kernel

### 3. Imported checks must run through product-owned governed execution

- `WP-1-Product-Governance-Check-Runner-v1` defines a bounded execution contract for imported checks, rubrics, and scripts
- it explicitly rejects raw shell bypass and repo-path coupling

Implication:
- even when repo-governance mechanisms are imported, execution authority must be re-expressed through Handshake runtime capabilities, recorder visibility, and workflow control

### 4. Overlay state should be mirrored into product runtime, not left in repo-side ledgers

- `WP-1-Governance-Workflow-Mirror-v1` defines per-WP gate state, activation traceability, and Flight Recorder governance events inside Handshake runtime
- it explicitly rejects treating imported repo-governance state as universal product authority

Implication:
- Handshake should project and normalize imported software-delivery governance state into product-owned records rather than continuing to operate directly on the repo-governance ledgers

### 5. Governance Pack is already framed as project-agnostic product capability

- `Handshake_Master_Spec_v02.180.md` `7.5.4.8 Governance Pack: Project-Specific Instantiation (HARD)` makes the governance kernel a project-parameterized product feature
- `WP-1-Governance-Pack-v1` reinforces that imported repo-governance assets are one software-delivery profile overlay inside a broader product capability

Implication:
- the end state is not "Handshake uses repo governance forever"
- the end state is "Handshake exports and runs governance packs, one of which may be the current software-delivery overlay"

## What This Means For The Research

The next architecture should not be synthesized against the current repo-governance kernel alone.
It should be synthesized against these product-owned target surfaces:

1. runtime governance state and workflow truth under `.handshake/gov/`
2. imported artifact registry for bounded software-delivery overlays
3. governed execution surface for imported checks and scripts
4. workflow mirror and recorder-visible governance projection
5. project-agnostic Governance Pack export and instantiation

## Harness Mechanism Mapping To Handshake Target Surfaces

### Runtime state, checkpointing, resume, and lifecycle

- `LangGraph`: checkpoint provenance, interrupt/resume, thread lifecycle, run durability controls
- `Open SWE`: deterministic thread identity, queued follow-up injection, reconnect-by-thread execution model
- `CrewAI`, `AutoGen`, `Semantic Kernel`, `PydanticAI`: promising second-wave references for checkpointable runtime state and explicit resume objects

Use for:
- product workflow engine
- durable run/thread identity
- resume and recovery semantics
- long-lived product-side control-plane state

### Policy, approvals, and authority boundaries

- `Letta`: typed policy rules, durable approval requests, copy-on-execute commit-back
- `BeeAI`: requirement compiler, inline permission requirements, tool governance
- `Cline`: semantic approval identities and file-backed hook policy surfaces
- `PydanticAI`: strong second-wave reference for approval-as-data and deferred tool execution

Use for:
- product capability gating
- approval stops
- policy-as-data
- validator and closeout authority boundaries

### Delegation, handoff, and work transfer

- `Roo Code`: isolated `new_task`, durable parent-child resume path
- `Open SWE`: queued steering and active-run-safe follow-up injection
- `OpenAI Swarm`: minimal lower-bound handoff semantics only

Use for:
- product micro-task transfer
- operator steer-next or relay surfaces
- explicit parent-child workflow linkage

### Evidence, replay, and artifact hygiene

- `SWE-agent`: replayable `.traj` bundles and structured post-action state capture
- `Cline`: checkpoint-to-message linkage and history reconstruction
- `TaskWeaver`: prompt-log capture, event taxonomy, typed execution or verification attachments
- `OpenHands`: `.pr/` artifact hygiene and structured resolver outputs

Use for:
- Flight Recorder-linked evidence
- replay-safe run artifacts
- temporary non-canonical artifact hygiene
- closeout evidence packaging

### Coordination substrate and bounded parallelism

- `AgentScope`: `MsgHub` scoped fan-out and pluggable state backends
- `PocketFlow`: explicit action-routed flow control, retry/fallback, shared-store-vs-params split
- `LangGraph`: stronger durable coordination substrate when explicit graph semantics are justified

Use for:
- work transfer and coordination channels
- bounded parallel fan-out
- explicit state versus compute separation

### Protocol and interop surfaces

- `BeeAI`: MCP, A2A, ACP adapter model
- `AgentScope`: MCP and A2A integration

Use for:
- future external control-plane edges
- protocol adapters around the product runtime rather than inside the core state model

## Guardrails For The Next Architecture Step

- do not port raw repo-governance ledgers into product runtime as the authoritative model
- do not let `.GOV/**` or repo `docs/**` reappear as live product runtime dependencies
- do not equate "software-delivery governance overlay" with "all Handshake governance"
- do not reuse repo-governance script surfaces as if they were already product execution contracts
- do preserve the repo-governance kernel as:
  - failure evidence
  - mechanism inventory
  - migration source
  - conformance reference for the software-delivery overlay only

## Recommended Next Sequence

1. finish only the targeted follow-up inspections that change architecture choices:
   - LangGraph concrete saver implementations for `copy/prune`
   - Open SWE restart and recovery behavior
   - Letta rule vocabulary mapped onto Handshake governance boundaries
   - strongest second-wave candidates for typed resume/approval objects: `PydanticAI`, `CrewAI`, `Semantic Kernel`, `AutoGen`
2. write `Handshake_Governance_Target_Architecture.md`
   - no implementation yet
   - define state model, authority model, execution identity model, approval model, evidence model, and interop boundaries
   - status: now captured in the working note and ready for deeper field-level refinement
3. map current repo-governance lessons and failures onto that target architecture
   - retain
   - adapt
   - delete
   - migrate
4. only then decide which repo-governance mechanisms should be ported into product code

## Working Decision

If the immediate question is "should we stop building on repo governance and focus on Handshake governance?" the answer is:

- yes, stop treating repo governance as the main architecture destination
- no, do not grow Handshake governance from the stale embedded repo-governance copy
- yes, use the repo-governance kernel as a bounded reference and mechanism source while the product-owned Handshake governance target is designed from the external harness research plus the existing product-side boundary work
