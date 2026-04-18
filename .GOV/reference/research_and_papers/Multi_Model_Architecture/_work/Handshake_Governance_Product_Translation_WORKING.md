# Handshake Governance Product Translation Working Note

Temporary working note that explains how the repo-governance inspection and external harness research combine with the Handshake product documents.

This is not a final implementation packet.
It is the bridge between:

- master-spec law
- product-reference framing
- external harness mechanism research
- repo-governance migration planning

## Purpose

- explain how the research translates into Handshake as a product feature set
- state which source is authoritative when sources differ
- map the research outcomes onto concrete Handshake runtime and projection surfaces
- make clear what changes in the current repo-governance testbed as migration proceeds

## Precedence

The interpretation order should be:

1. `Handshake_Master_Spec_v02.180.md`
2. `HANDSHAKE_PRODUCT_REFERENCE.md`
3. research working notes in this `_work` folder
4. current repo-governance kernel and artifacts

Meaning:

- the master spec defines the law, boundaries, and mandatory product surfaces
- the product reference is the fast index into those surfaces, but it is reference-only
- the research notes choose mechanism shape where the spec leaves implementation latitude
- the current repo-governance kernel is migration input and failure evidence, not product authority

## Translation Formula

The translation from research to product should be read as:

- master spec defines invariants and product-owned control-plane surfaces
- product reference identifies the main runtime primitives and relationships
- external harness research contributes mechanism choices that fit inside those surfaces
- repo-governance inspection contributes failure cases, migration pressure, and compatibility constraints

So the product outcome is not:

- "import repo governance into Handshake"
- "embed a third-party harness into Handshake"

The product outcome is:

- Handshake-native software-delivery governance overlay
- running inside Handshake workflow, capability, recorder, and projection systems
- with imported repo-governance artifacts treated as overlay definition, source material, or migration input

## Source Stack Used By This Translation

- master authority:
  - `.GOV/spec/Handshake_Master_Spec_v02.180.md`
- product reference lens:
  - `.GOV/spec/HANDSHAKE_PRODUCT_REFERENCE.md`
- repo-governance comparison and migration pressure:
  - `.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Repo_Governance_Harness_Comparison_WORKING.md`
- harness mechanism extraction:
  - `.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Harness_Adoption_Extraction_WORKING.md`
- typed runtime and approval follow-up:
  - `.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Typed_Runtime_Resume_Approval_WORKING.md`
- target architecture synthesis:
  - `.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Handshake_Governance_Target_Architecture_WORKING.md`
- field-level schema sketch:
  - `.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Handshake_Governance_Runtime_Schema_Sketch_WORKING.md`
- merge-oriented mini spec draft:
  - `.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Handshake_Governance_Mini_Spec_WORKING.md`
- merged unified technical draft:
  - `.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Handshake_Governance_Unified_Technical_Spec_WORKING.md`

## Product Translation Map

### 1. Governance boundary and runtime storage

Master-spec anchor:

- product code must not treat repo `/.GOV/**` as live runtime authority
- runtime governance state lives in product-owned storage, default `.handshake/gov/`
- product governance snapshot is deterministic export derived from canonical repo governance inputs

Research contribution:

- repo-governance comparison showed that live file authority plus side ledgers creates truth drift
- harness research reinforced the need for durable runtime-owned state, explicit resume, and storage-backed lineage

Product consequence:

- Handshake stores live software-delivery governance state in product-owned runtime records
- repo `/.GOV/**` becomes:
  - canonical governance workspace for repo-side governance
  - import source for software-delivery overlay artifacts
  - export or snapshot source where the spec explicitly says so

Migration consequence for current repo governance:

- current packet, board, runtime status, and gate files stop competing as product truth
- they become imported source material, compatibility mirrors, evidence, or export inputs

### 2. Structured work contracts instead of mutable packet-ledger truth

Master-spec anchor:

- shared base envelope
- compact summary contract
- Work Packet and related records are structured-record-first
- readable Markdown remains a mirror or sidecar, not the authority

Research contribution:

- repo-governance inspection identified packet-ledger overload, repeated routing rediscovery, and closeout reconciliation pain
- harness research showed the value of durable execution contracts, checkpointable work identity, and summary-first operating views

Product consequence:

- Handshake product runtime should hold the software-delivery contract in canonical records such as:
  - `GovernanceWorkContractRecord`
  - `GovernanceWorkflowBindingRecord`
- DCC, Task Board, and other surfaces should load summary-first, then drill down into canonical detail

Migration consequence for current repo governance:

- task packet Markdown survives as:
  - source artifact
  - readable mirror
  - import compatibility surface
- but not as the mutable operational truth ledger

### 3. One routing law across DCC, Task Board, Role Mailbox, and model tiers

Master-spec anchor:

- one shared `workflow_state_family`
- one shared `queue_reason_code`
- one shared `allowed_action_ids`
- transition rules, queue automation rules, and executor eligibility policies
- local-small-model routing must depend on explicit state and summary, not lane names or prose

Research contribution:

- the harness work was most valuable where it clarified durable execution identity, resume, fan-out, approval posture, and queue-safe coordination
- the repo-governance comparison showed the cost of routing via board labels, folder conventions, and shell choreography instead of durable state

Product consequence:

- Handshake should use one routing substrate across:
  - DCC queues
  - Task Board layouts
  - Role Mailbox-linked waits
  - local small model execution
  - cloud escalation or reviewer paths
- software-delivery research narrows and applies the shared taxonomy, but does not replace it

Migration consequence for current repo governance:

- lane names, thread order, and command rituals stop being the routing authority
- repo-specific readiness concepts become profile extensions or view labels over the base workflow law

### 4. Typed approvals, resumable governed actions, and explicit handoff transcription

Master-spec anchor:

- governed action descriptors
- explicit approval boundaries
- mailbox actions may recommend or prepare linked work changes but do not mutate linked authority directly
- DCC is the control surface for resume, reroute, and recovery

Research contribution:

- `PydanticAI`, `Letta`, `AutoGen`, `CrewAI`, and `LangGraph` gave the strongest mechanism ideas for deferred execution, approval stops, checkpoint lineage, and resumable state
- repo-governance inspection showed the pain of implicit approvals, session-control residue, and transcript-dependent closeout

Product consequence:

- Handshake product should carry approval and side-effect posture in typed records such as:
  - `GovernedActionRequest`
  - `GovernedActionResolution`
  - `GovernanceCheckpointRecord`
- mailbox summaries, announce-back traffic, and handoff notes should change authoritative work only through governed transcription

Migration consequence for current repo governance:

- session-control ledgers and handoff prose remain useful evidence
- but they stop being the mechanism by which runtime truth is inferred

### 5. Validator gates become product-native validation records

Master-spec anchor:

- runtime gate state lives under `.handshake/gov/validator_gates/...`
- Governance Check Runner defines bounded `governance.check.run`
- `CheckResult` is typed and limited to `PASS`, `FAIL`, `BLOCKED`, `ADVISORY_ONLY`, `UNSUPPORTED`
- FR visibility is mandatory

Research contribution:

- external harnesses helped separate tool admission, bounded execution, and replay-friendly evidence
- repo-governance inspection showed that gate state, validator results, and closeout evidence are too brittle when scattered across files and scripts

Product consequence:

- validator posture should be represented as a dedicated runtime record family:
  - `GovernanceValidatorGateRecord`
- that record bridges:
  - legacy gate compatibility status
  - bounded check-runner result rollups
  - evidence refs
  - workflow-blocking consequences

Migration consequence for current repo governance:

- legacy `.GOV/validator_gates/{WP_ID}.json` becomes import or compatibility input
- product runtime owns active gate posture, non-pass lineage, and check-result rollups

### 6. DCC, Task Board, and Role Mailbox stay projections over the same truth

Master-spec anchor:

- Dev Command Center is the canonical projection and control surface
- Task Board is a human-readable synchronization mirror, not execution authority
- Role Mailbox is structured collaboration and triage, not linked work authority
- mirror authority mode and reconciliation action must stay visible

Research contribution:

- repo-governance inspection exposed the cost of too many quasi-authoritative views
- harness research reinforced summary-first operation, typed event surfaces, and queryable recovery state

Product consequence:

- DCC becomes the main surface for:
  - work contract state
  - workflow routing
  - validator gate posture
  - pending approvals
  - checkpoint and replay views
- Task Board stays a planning and visibility mirror
- Role Mailbox stays collaboration and async routing, with explicit transcription boundaries

Migration consequence for current repo governance:

- `TASK_BOARD.md`, thread artifacts, receipts, and runtime-status files remain useful as mirrors or evidence
- but they no longer compete with canonical runtime records for authority

### 7. Governance Pack turns current repo governance into one overlay profile, not the product core

Master-spec anchor:

- Governance Pack is project-parameterized
- export/import is product capability
- product governance snapshot is deterministic and leak-safe
- imported governance is additive, not universal authority

Research contribution:

- repo-governance comparison reframed the current kernel as a failure-rich software-delivery testbed
- harness research showed what a cleaner software-delivery overlay needs in order to be portable and bounded

Product consequence:

- Handshake as a product stays broader than software delivery
- software-delivery governance becomes one overlay profile or pack running on top of shared product primitives
- the current repo-governance kernel is a strong candidate migration source and conformance reference for that overlay

Migration consequence for current repo governance:

- the current kernel should be mined, normalized, and eventually exportable as a Governance Pack or equivalent overlay definition
- it should not remain the product kernel itself

## How The Product Reference Fits

`HANDSHAKE_PRODUCT_REFERENCE.md` is useful here as the fast lookup layer for the product primitives and surfaces the research needs to land on.

It helps answer:

- where workflow truth should live
- where model-session identity should live
- where DCC projection should land
- where Flight Recorder evidence belongs
- where governance import or execution surfaces already exist conceptually

But it does not outrank the master spec.
When the product reference and the research need arbitration, the master spec wins.

## What The Research Changes In Practice

The research does not ask the product to become a repo-governance shell.
It asks the product to add a stronger software-delivery overlay on top of the existing Handshake substrate.

Practically, that means:

- stronger canonical runtime records
- explicit governed-action and approval handling
- dedicated validator gate records
- summary-first projection surfaces
- replay-visible and recorder-visible evidence
- imported overlay definitions instead of raw repo-path coupling

## What Stays General To Handshake

This work does not reduce Handshake to software delivery.

The master spec already defines generic surfaces:

- structured collaboration artifacts
- workflow engine
- Dev Command Center
- Role Mailbox
- capability gates
- Flight Recorder
- project-profile extensions

The research only specializes those surfaces for one overlay:

- software-delivery governance

That is why the translation is product-safe.
It strengthens a reusable substrate instead of hardcoding repo-governance assumptions into the entire product.

## Working Synthesis

The right synthesis is:

- master spec supplies the law
- product reference supplies the fast product map
- harness research supplies mechanism choices
- repo-governance inspection supplies failure evidence and migration constraints

The resulting implementation target is:

- a Handshake-native software-delivery governance overlay
- with runtime truth in product-owned records
- imported repo-governance assets treated as overlay definition and migration material
- DCC, Task Board, and Role Mailbox kept as projections or collaboration surfaces over the same canonical truth

## Immediate Follow-On Use

This note should now be used as the bridge when making the next decisions:

1. which proposed runtime primitives become actual product primitives or schema entries
2. which current repo-governance artifacts become:
   - imported overlay data
   - readable mirrors
   - evidence
   - compatibility-only exports
3. which product surfaces need first implementation priority:
   - governed actions
   - validator gate records
   - DCC projection
   - overlay import and check execution

The direct next drafting layer above this note is:

- `.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Handshake_Governance_Mini_Spec_WORKING.md`
- `.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Handshake_Governance_Unified_Technical_Spec_WORKING.md`
