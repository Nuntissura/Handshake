# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.
- For any stub covering current-spec Phase 1 roadmap additions (`[ADD v<current>]`), `ROADMAP_ADD_COVERAGE` MUST enumerate the exact spec line numbers so governance checks can verify no additions were missed.

---

# Work Packet Stub: WP-1-Dev-Command-Center-Structured-Artifact-Viewer-v1

## STUB_METADATA
- WP_ID: WP-1-Dev-Command-Center-Structured-Artifact-Viewer-v1
- BASE_WP_ID: WP-1-Dev-Command-Center-Structured-Artifact-Viewer
- CREATED_AT: 2026-03-10T00:19:00.642Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: FRONTEND
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: MEDIUM
- BUILD_ORDER_DEPENDS_ON: WP-1-Dev-Command-Center-MVP, WP-1-Structured-Collaboration-Artifact-Family
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.169.md 7.6.3 (Phase 1) -> [ADD v02.169] Canonical-to-mirror reconciliation and drift governance
- ROADMAP_ADD_COVERAGE: SPEC=v02.169; PHASE=7.6.3; LINES=46775
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.169.md 10.11.5.17 Structured Work Records, Notes, and Collaboration Inbox [ADD v02.169]
  - Handshake_Master_Spec_v02.169.md 10.11.5.18 Projected Boards and Queue Layouts over Structured Records [ADD v02.169]
  - Handshake_Master_Spec_v02.169.md 10.11.5.19 Canonical Records, Markdown Mirrors, and Drift Reconciliation [ADD v02.169]

## INTENT (DRAFT)
- What: Build the Dev Command Center typed-field viewers and derived board or queue layouts over canonical structured collaboration artifacts.
- Why: Operators should inspect and steer work through clean field-based surfaces instead of raw Markdown or raw JavaScript Object Notation blobs, and future Jira-like layouts should remain just alternate projections over the same authority.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Work Packet, Micro-Task, Task Board, and Role Mailbox typed-field viewers.
  - Two or more derived layouts such as kanban, list, queue, or roadmap from the same structured records.
  - Explicit display of authoritative fields versus Markdown mirrors.
  - Raw JavaScript Object Notation drilldown as an advanced view, not the default operator surface.
- OUT_OF_SCOPE:
  - Defining the canonical backend artifact schemas.
  - Full external board-platform integration.

## UI_UX_SKETCH (DRAFT)
- Principle: prefer enumerating "too many" controls early, consolidate later.
- Include minimalistic in-UI explainers (prefer hover tooltips), and ensure tooltips are accessible (hover + keyboard focus; dismissible; avoid violating WCAG 1.4.13).
- UI_SURFACES:
  - Dev Command Center work record panel
  - Role Mailbox triage queue
  - Board-layout switcher
  - Raw-record inspector drawer
- UI_CONTROLS (buttons/dropdowns/inputs):
  - Control: View layout switcher | Type: segmented control | Tooltip: Switch between derived board or queue layouts without changing authority. | Notes: kanban, list, queue, roadmap
  - Control: Show canonical fields | Type: toggle | Tooltip: Show authoritative structured fields before mirror content. | Notes: default on
  - Control: Mirror status badge | Type: status chip | Tooltip: Shows whether the Markdown mirror is synchronized, stale, or advisory. | Notes: read-only state
- UI_STATES (empty/loading/error):
  - Empty structured record
  - Mirror stale
  - Layout unavailable because required fields are missing
- UI_MICROCOPY_NOTES (labels, helper text, hover explainers):
  - Canonical structured fields
  - Derived Markdown mirror
  - Advisory human edit pending normalization
  - Manual resolution required before regenerate

## RESEARCH_SCOUTING (DRAFT)
- RESEARCH_CURRENCY_REQUIRED: YES
- TARGET_BUCKETS:
  - BIG_TECH
  - UNIVERSITY|PAPER
  - GITHUB|OSS_DOC
- SEARCH_SEEDS:
  - Jira board columns and issue field projections
  - GitHub Projects fields and views
  - typed metadata plus readable narrative split
  - project catalog entity pages and descriptor viewers
- CANDIDATE_SOURCES:
  - Source: Atlassian support docs for Kanban boards | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-10T01:25:00Z | URL: https://support.atlassian.com/jira-software-cloud/docs/configure-columns/ | Why: board columns show how layout can map onto state without becoming the only source of truth
  - Source: GitHub Projects fields docs | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-10T01:26:00Z | URL: https://docs.github.com/issues/planning-and-tracking-with-projects/understanding-fields/about-fields | Why: field-driven projections and multiple views over the same item set
  - Source: GitHub roadmap layout docs | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-10T01:27:00Z | URL: https://docs.github.com/issues/planning-and-tracking-with-projects/customizing-views-in-your-project/customizing-the-roadmap-layout | Why: roadmap is a layout over existing item fields, not a second issue authority
  - Source: Model Cards for Model Reporting | Kind: UNIVERSITY|PAPER | Date: 2018-10-09 | Retrieved: 2026-03-10T01:28:00Z | URL: https://arxiv.org/abs/1810.03993 | Why: readable narrative can be paired with structured metadata and explicit context instead of raw opaque blobs
  - Source: Hugging Face model cards | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-10T01:29:00Z | URL: https://huggingface.co/docs/hub/model-cards | Why: model cards are a practical example of metadata-first readable documentation rendered for operators
  - Source: Backstage descriptor format | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-10T01:30:00Z | URL: https://backstage.io/docs/features/software-catalog/descriptor-format | Why: typed descriptors plus catalog entity pages map well to structured collaboration viewers

## RESEARCH_DECISIONS (DRAFT)
- ADOPT:
  - Source: GitHub Projects fields docs | Pattern: board, list, and roadmap layouts derived from stable field sets | Why: Dev Command Center should switch layouts without changing record authority
- ADAPT:
  - Source: Hugging Face model cards | Pattern: structured metadata paired with readable narrative | Why: Dev Command Center should show typed canonical fields first, then readable mirrors and notes as linked secondary surfaces
- REJECT:
  - Source: Atlassian support docs for Kanban boards | Pattern: layout movement or column placement treated as implicit workflow authority | Why: Handshake board, queue, and roadmap layouts must remain projections over canonical records and explicit edit actions

## GITHUB_PROJECT_SCOUTING (DRAFT)
- SEARCH_QUERIES:
  - backstage catalog entity page structured metadata viewer
  - langgenius dify workflow history ui
  - openhands event timeline ui repo
- MATCHED_PROJECTS:
  - Repo: backstage/backstage | Intent: ARCH_PATTERN | Decision hint: ADAPT | Impact hint: UI_ENRICHMENT | Notes: plugin-style entity pages and descriptor-backed catalog views fit typed Work Packet and Task Board panels
  - Repo: langgenius/dify | Intent: UI_PATTERN | Decision hint: ADAPT | Impact hint: UI_ENRICHMENT | Notes: workflow-history and node-log panels are useful patterns for drilldown and replay-safe operator inspection
  - Repo: OpenHands/OpenHands | Intent: UI_PATTERN | Decision hint: TRACK_ONLY | Impact hint: UI_ENRICHMENT | Notes: session timeline and event-backed work views are relevant, but broader than this stub's typed viewer scope

## PILLAR_FORCE_MULTIPLIERS (DRAFT)
- TOUCHED_OR_UNKNOWN_PILLARS:
  - PILLAR: Development Operations | STATUS: TOUCHED | NOTES: operators need layout switching, field provenance, and mirror-drift visibility without leaving the Dev Command Center | Stub follow-up: THIS_STUB
  - PILLAR: Human Collaboration | STATUS: TOUCHED | NOTES: handoff notes, mailbox summaries, and packet narratives remain readable without confusing the true authority | Stub follow-up: THIS_STUB
  - PILLAR: Governance and Audit | STATUS: TOUCHED | NOTES: the viewer should expose why a record is trusted, stale, or normalization-required before action | Stub follow-up: THIS_STUB

## MECHANICAL_ENGINE_SCOUTING (DRAFT)
- TOUCHED_OR_UNKNOWN_ENGINES:
  - ENGINE: Dev Command Center structured viewer projector | ENGINE_ID: dev_command_center.structured_viewer | STATUS: TOUCHED | NOTES: renders canonical fields, summaries, mirrors, and drift state in one governed surface | Stub follow-up: THIS_STUB
  - ENGINE: Task Board layout projector | ENGINE_ID: task_board.layout_projection | STATUS: TOUCHED | NOTES: supports kanban, list, queue, and roadmap layouts without mutating authority | Stub follow-up: THIS_STUB

## PRIMITIVES_AND_MATRIX_NOTES (DRAFT)
- PRIMITIVES_TOUCHED:
  - PRIM-TrackedWorkPacket
  - PRIM-TrackedMicroTask
  - PRIM-TaskBoardEntry
  - PRIM-RoleMailboxBodyV0_5
  - PRIM-MirrorAuthorityMode
  - PRIM-MirrorReconciliationAction
  - PRIM-MarkdownMirrorContractV1
- PRIMITIVE_MATRIX_COMBO_CANDIDATES:
  - Edge: FEAT-DEV-COMMAND-CENTER -> FEAT-TASK-BOARD | ROI: H | Effort: M | Notes: one typed viewer should back kanban, queue, and roadmap layouts while surfacing field provenance
  - Edge: FEAT-DEV-COMMAND-CENTER -> FEAT-ROLE-MAILBOX | ROI: M | Effort: M | Notes: mailbox triage should reuse the same mirror badges, diff summary, and raw-record drilldown pattern

## FORCE_MULTIPLIER_HYPOTHESES (DRAFT)
- CANDIDATES:
  - Combo: typed field viewer plus mirror badges plus multi-layout board switcher | Pillars: Development Operations, Human Collaboration, Governance and Audit | Mechanical: dev_command_center.structured_viewer, task_board.layout_projection | Primitives/Features: PRIM-TrackedWorkPacket, PRIM-TaskBoardEntry, PRIM-MarkdownMirrorContractV1, FEAT-DEV-COMMAND-CENTER, FEAT-TASK-BOARD | Resolution hint: IN_THIS_STUB | Notes: one viewer architecture can power packet detail, queue triage, and future Jira-like experimentation without duplicating data logic

## EXISTING_CAPABILITY_SCOUTING (DRAFT)
- MATCHED_STUBS:
  - Artifact: WP-1-Markdown-Mirror-Sync-Drift-Guard-v1 | Intent: PARTIAL | PrimitiveIndex: PARTIAL | Matrix: PARTIAL | UI: PARTIAL | Resolution hint: KEEP_SEPARATE | Notes: mirror reconciliation semantics belong there; this stub consumes those semantics in typed viewer flows
  - Artifact: WP-1-Structured-Collaboration-Artifact-Family-v1 | Intent: PARTIAL | PrimitiveIndex: PARTIAL | Matrix: PARTIAL | UI: NONE | Resolution hint: KEEP_SEPARATE | Notes: canonical records and file families are prerequisites, but they do not define the operator surface
  - Artifact: WP-1-Structured-Collaboration-Schema-Registry-v1 | Intent: PARTIAL | PrimitiveIndex: PARTIAL | Matrix: PARTIAL | UI: NONE | Resolution hint: KEEP_SEPARATE | Notes: viewer must trust registry outputs instead of inventing its own field interpretation
- MATCHED_ACTIVE_OR_DONE_PACKETS:
  - Artifact: WP-1-Role-Mailbox-v1 | Status: VALIDATED | Intent: PARTIAL | PrimitiveIndex: PARTIAL | Matrix: PARTIAL | UI: PARTIAL | CodeReality: PARTIAL | Resolution hint: KEEP_SEPARATE | Notes: existing mailbox flows prove the need for a better structured viewer, but the packet predates typed canonical-record rendering
- CODE_REALITY_HINTS:
  - Path: .GOV/roles_shared/TASK_BOARD.md | Covers: ui-intent | Notes: readable work views already exist, but they are not yet backed by a typed product viewer with explicit field provenance

## APPENDIX_MAINTENANCE_NOTES (DRAFT)
- FEATURE_REGISTRY:
  - FEAT-DEV-COMMAND-CENTER, FEAT-ROLE-MAILBOX, FEAT-TASK-BOARD, and FEAT-WORK-PACKET-SYSTEM gain typed-viewer and mirror-badge notes
- PRIMITIVE_INDEX:
  - Reuse PRIM-TrackedWorkPacket, PRIM-TrackedMicroTask, PRIM-TaskBoardEntry, and PRIM-RoleMailboxBodyV0_5; add mirror-contract primitives when viewer scope expands
- UI_GUIDANCE:
  - Dev Command Center, Role Mailbox, Task Board, and Work Packet views gain entry points, telemetry, and test expectations for typed fields, mirror badges, and raw-record drilldown
- INTERACTION_MATRIX:
  - Deepen IMX-105, IMX-108, IMX-109, IMX-110, IMX-116, IMX-117, IMX-119, and IMX-120 where Dev Command Center consumes mirror contracts and canonical field provenance

## ACCEPTANCE_CRITERIA (DRAFT)
- Dev Command Center can render typed Work Packet, Micro-Task, Task Board, and Role Mailbox views from canonical structured records.
- At least two different board or queue layouts can be derived from the same record set without mutating authority.
- The operator can distinguish canonical fields, mirror content, and mirror-drift state from one screen.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on the canonical structured collaboration artifact family.
- Depends on base Dev Command Center implementation and structured record availability in backend APIs.

## RISKS / UNKNOWNs (DRAFT)
- Risk: raw JavaScript Object Notation fallback becomes the default surface. Mitigation: typed-field-first rendering and advanced inspector separation.
- Risk: board layouts smuggle in state edits. Mitigation: explicit governed record edits only, with visible field provenance.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Draft research scouting across BIG_TECH, UNIVERSITY/PAPER, and GITHUB/OSS sources unless the work is strictly internal/mechanical.
- [ ] Draft pillar force multipliers and primitive-matrix combo candidates; create extra stubs instead of guessing.
- [ ] If refinement is likely to grow the primitive index, feature registry, UI guidance, or interaction matrix, treat activation as a spec-version update flow first, then re-activate the WP against the new `SPEC_CURRENT`.
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Dev-Command-Center-Structured-Artifact-Viewer-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Dev-Command-Center-Structured-Artifact-Viewer-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
