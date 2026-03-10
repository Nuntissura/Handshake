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

# Work Packet Stub: WP-1-Dev-Command-Center-Layout-Projection-Registry-v1

## STUB_METADATA
- WP_ID: WP-1-Dev-Command-Center-Layout-Projection-Registry-v1
- BASE_WP_ID: WP-1-Dev-Command-Center-Layout-Projection-Registry
- CREATED_AT: 2026-03-10T01:53:38.217Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: FRONTEND
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: MEDIUM
- BUILD_ORDER_DEPENDS_ON: WP-1-Dev-Command-Center-MVP, WP-1-Dev-Command-Center-Structured-Artifact-Viewer, WP-1-Structured-Collaboration-Artifact-Family, WP-1-Structured-Collaboration-Schema-Registry
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.170.md 7.6.3 (Phase 1) -> [ADD v02.170] Dev Command Center typed viewer, board layout, and queue projection
- ROADMAP_ADD_COVERAGE: SPEC=v02.170; PHASE=7.6.3; LINES=46844
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.170.md 10.11.5.17 Structured Work Records, Notes, and Collaboration Inbox [ADD v02.170]
  - Handshake_Master_Spec_v02.170.md 10.11.5.18 Projected Boards and Queue Layouts over Structured Records [ADD v02.170]
  - Handshake_Master_Spec_v02.170.md 10.11.5.20 Typed Viewer Presets, Lane Definitions, and Governed Actions [ADD v02.170]

## INTENT (DRAFT)
- What: Define and later implement the registry of Dev Command Center layout presets, lane definitions, grouping rules, and governed action bindings for board, queue, list, roadmap, inbox-triage, and execution-queue operating views.
- Why: Handshake needs a typed layout layer so Jira-like or novel operator boards can evolve without turning drag behavior, queue ordering, or quick actions into hidden workflow authority.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Versioned `DevCommandCenterViewPresetV1` preset registry for board, queue, list, roadmap, inbox-triage, and execution-queue layouts.
  - Stable `TaskBoardLaneDefinitionV1` and `ProjectionActionBindingV1` registration, validation, and lookup.
  - Dev Command Center layout switching that reveals the authoritative fields, grouping keys, lane rules, and governed action previews behind each view.
  - Local-small-model execution queues and mailbox triage layouts that stay compact-summary-first and field-addressable.
- OUT_OF_SCOPE:
  - Replacing the canonical structured collaboration schema family.
  - Direct synchronization with external products such as Jira.
  - Ad hoc layout-local mutations that bypass workflow, approval, or evidence rules.

## UI_UX_SKETCH (DRAFT)
- Principle: prefer enumerating "too many" controls early, consolidate later.
- Include minimalistic in-UI explainers (prefer hover tooltips), and ensure tooltips are accessible (hover + keyboard focus; dismissible; avoid violating WCAG 1.4.13).
- UI_SURFACES:
  - Dev Command Center preset switcher and view header
  - Layout registry inspector
  - Lane-definition editor and preview
  - Governed action preview dialog
  - Local-small-model execution queue
  - Role Mailbox inbox-triage preset view
- UI_CONTROLS (buttons/dropdowns/inputs):
  - Control: View preset switcher | Type: segmented control | Tooltip: Switch between board, queue, list, roadmap, inbox triage, and execution queue layouts without hiding the authoritative grouping rules. | Notes: default to last safe preset per operator and record scope
  - Control: Lane definition inspector | Type: drawer toggle | Tooltip: Show the field match rules, sort keys, and work-in-progress caps behind the current lane layout. | Notes: read-only outside registry management flows
  - Control: Action preview | Type: modal dialog | Tooltip: Preview the record ids, field paths, workflow ids, approvals, and evidence requirements behind a quick action, drag action, or bulk action. | Notes: required before authoritative mutation
  - Control: Queue grouping selector | Type: dropdown | Tooltip: Regroup queues by status, expected response, escalation posture, or assigned role using preset-safe fields only. | Notes: unavailable when preset is fixed
- UI_STATES (empty/loading/error):
  - No compatible preset for current record scope
  - Preset exists but required fields are missing
  - Action binding requires approval before execution
  - Layout degraded to base-envelope fallback
- UI_MICROCOPY_NOTES (labels, helper text, hover explainers):
  - Current view preset
  - Governs grouping only until an explicit action binding runs
  - Preview authoritative mutation
  - Compact summary first for local small model routing

## RESEARCH_SCOUTING (DRAFT)
- RESEARCH_CURRENCY_REQUIRED: YES
- TARGET_BUCKETS:
  - BIG_TECH
  - UNIVERSITY|PAPER
  - GITHUB|OSS_DOC
- SEARCH_SEEDS:
  - Jira board columns and workflow mapping
  - GitHub Projects field-driven views
  - Linear issue triage low-noise layouts
  - Plane board and cycle planning layouts
  - Backstage plugin-backed entity pages
  - OpenHands and Dify operator workflow drilldown
- CANDIDATE_SOURCES:
  - Source: Atlassian support docs for Kanban board columns | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-10T02:05:00Z | URL: https://support.atlassian.com/jira-software-cloud/docs/configure-columns/ | Why: columns and statuses show how layout can map onto governed workflow without every column becoming a source of truth.
  - Source: GitHub Projects fields docs | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-10T02:06:00Z | URL: https://docs.github.com/issues/planning-and-tracking-with-projects/understanding-fields/about-fields | Why: multiple layouts can derive from stable item fields and typed metadata.
  - Source: GitHub roadmap layout docs | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-10T02:06:30Z | URL: https://docs.github.com/issues/planning-and-tracking-with-projects/customizing-views-in-your-project/customizing-the-roadmap-layout | Why: roadmap posture is a projection over item fields, not a separate issue authority.
  - Source: Linear issue tracking docs | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-10T02:07:00Z | URL: https://linear.app/docs/issue-tracking | Why: compact triage and low-noise issue state are useful for Handshake queue presets.
  - Source: Plane developer docs | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-10T02:08:00Z | URL: https://developers.plane.so/api-reference/project/project-create | Why: open-source issue and cycle planning surfaces show how structured planning records can back multiple board patterns.
  - Source: Backstage descriptor format | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-10T02:09:00Z | URL: https://backstage.io/docs/features/software-catalog/descriptor-format | Why: typed descriptors plus configurable entity pages fit registry-driven layout rendering.
  - Source: OpenHands conversation architecture | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-10T02:10:00Z | URL: https://docs.openhands.dev/sdk/arch/conversation | Why: operator surfaces need explicit lifecycle state and event-backed drilldown rather than opaque panel-local state.
  - Source: Dify history and logs docs | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-10T02:11:00Z | URL: https://docs.dify.ai/en/use-dify/debug/history-and-logs | Why: node-level and run-level drilldown are good patterns for queue and execution views.

## RESEARCH_DECISIONS (DRAFT)
- ADOPT:
  - Source: GitHub Projects fields docs | Pattern: board, list, and roadmap views derived from stable fields | Why: Handshake layouts should switch projection without changing authority.
- ADAPT:
  - Source: Linear issue tracking docs | Pattern: compact triage and low-noise state presentation | Why: local-small-model and mailbox queues need concise operator surfaces without dropping field provenance.
- REJECT:
  - Source: Atlassian support docs for Kanban board columns | Pattern: treating a board column move as implicit workflow authority | Why: Handshake drag behavior must stay explicit through governed action bindings and previewable mutations.

## GITHUB_PROJECT_SCOUTING (DRAFT)
- SEARCH_QUERIES:
  - openhands workflow view
  - dify workflow history ui
  - backstage entity page metadata layout
  - plane issue board open source
- MATCHED_PROJECTS:
  - Repo: OpenHands/OpenHands | Intent: UI_PATTERN | Decision hint: ADAPT | Impact hint: UI_ENRICHMENT | Notes: event-backed session and action views are relevant for queue drilldown and governed action previews.
  - Repo: langgenius/dify | Intent: UI_PATTERN | Decision hint: ADAPT | Impact hint: UI_ENRICHMENT | Notes: run history and node drilldown patterns are useful for execution queues and replay-safe work views.
  - Repo: backstage/backstage | Intent: ARCH_PATTERN | Decision hint: ADOPT | Impact hint: SPEC_UPDATE | Notes: plugin-style, descriptor-backed pages fit a layout registry better than a hard-coded mega-screen.
  - Repo: makeplane/plane | Intent: UI_PATTERN | Decision hint: ADAPT | Impact hint: UI_ENRICHMENT | Notes: open planning-board patterns are useful for roadmap and queue presets.

## PILLAR_FORCE_MULTIPLIERS (DRAFT)
- TOUCHED_OR_UNKNOWN_PILLARS:
  - PILLAR: Development Operations | STATUS: TOUCHED | NOTES: typed board, queue, roadmap, and execution views make the Dev Command Center a real operating surface rather than a raw-record browser. | Stub follow-up: THIS_STUB
  - PILLAR: Human Collaboration | STATUS: TOUCHED | NOTES: mailbox triage and governed next-action previews reduce handoff ambiguity across roles and model tiers. | Stub follow-up: THIS_STUB
  - PILLAR: Governance and Audit | STATUS: TOUCHED | NOTES: explicit action previews keep board gestures and queue actions inspectable before state changes. | Stub follow-up: THIS_STUB

## MECHANICAL_ENGINE_SCOUTING (DRAFT)
- TOUCHED_OR_UNKNOWN_ENGINES:
  - ENGINE: Dev Command Center layout registry | ENGINE_ID: dev_command_center.layout_registry | STATUS: TOUCHED | NOTES: stores and validates preset ids, grouping rules, lane definitions, and layout-specific fallbacks. | Stub follow-up: THIS_STUB
  - ENGINE: Projection action router | ENGINE_ID: dev_command_center.projection_action_router | STATUS: TOUCHED | NOTES: maps drag, quick, and bulk actions to explicit field or workflow mutations with preview and approval posture. | Stub follow-up: THIS_STUB

## PRIMITIVES_AND_MATRIX_NOTES (DRAFT)
- PRIMITIVES_TOUCHED:
  - PRIM-DevCommandCenterLayoutKind
  - PRIM-ProjectionActionBindingV1
  - PRIM-TaskBoardLaneDefinitionV1
  - PRIM-DevCommandCenterViewPresetV1
- PRIMITIVE_MATRIX_COMBO_CANDIDATES:
  - Edge: FEAT-DEV-COMMAND-CENTER -> FEAT-TASK-BOARD | ROI: H | Effort: M | Notes: lane definitions and preset ids keep board movement explicit.
  - Edge: FEAT-ROLE-MAILBOX -> FEAT-MICRO-TASK-EXECUTOR | ROI: H | Effort: M | Notes: expected-response posture should unblock or block execution queues deterministically.
  - Edge: FEAT-WORK-PACKET-SYSTEM -> FEAT-DEV-COMMAND-CENTER | ROI: H | Effort: M | Notes: governed next-action previews need packet-aware routing and evidence posture.

## FORCE_MULTIPLIER_HYPOTHESES (DRAFT)
- CANDIDATES:
  - Combo: Typed layout registry plus governed action router | Pillars: Development Operations, Human Collaboration, Governance and Audit | Mechanical: dev_command_center.layout_registry, dev_command_center.projection_action_router | Primitives/Features: PRIM-DevCommandCenterViewPresetV1, PRIM-ProjectionActionBindingV1, FEAT-DEV-COMMAND-CENTER, FEAT-TASK-BOARD, FEAT-ROLE-MAILBOX | Resolution hint: IN_THIS_STUB | Notes: one registry can serve board, queue, roadmap, mailbox triage, and local-small-model execution views without duplicating layout logic.

## EXISTING_CAPABILITY_SCOUTING (DRAFT)
- MATCHED_STUBS:
  - Artifact: WP-1-Dev-Command-Center-Structured-Artifact-Viewer-v1 | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: PARTIAL | UI: PARTIAL | Resolution hint: KEEP_SEPARATE | Notes: viewer rendering exists as a related concern, but layout registry and action-binding orchestration deserve a separate implementation track.
  - Artifact: WP-1-Markdown-Mirror-Sync-Drift-Guard-v1 | Intent: DISTINCT | PrimitiveIndex: PARTIAL | Matrix: PARTIAL | UI: PARTIAL | Resolution hint: KEEP_SEPARATE | Notes: mirror governance feeds view badges and warnings, but not the layout registry itself.
- MATCHED_ACTIVE_OR_DONE_PACKETS:
  - Artifact: WP-1-Locus-Phase1-Integration-Occupancy-v1 | Status: VALIDATED | Intent: DISTINCT | PrimitiveIndex: N/A | Matrix: PARTIAL | UI: NONE | CodeReality: PARTIAL | Resolution hint: KEEP_SEPARATE | Notes: occupancy and work tracking are upstream authority inputs, not the operating layout registry itself.
- CODE_REALITY_HINTS:
  - Path: src/features/dev-command-center | Covers: ui-intent | Notes: expected home for typed viewers and preset switching when implementation starts.
  - Path: .GOV/roles_shared/TASK_BOARD.md | Covers: combo | Notes: current board is a governance mirror and should remain a projection source, not the layout authority.

## APPENDIX_MAINTENANCE_NOTES (DRAFT)
- FEATURE_REGISTRY:
  - Keep FEAT-DEV-COMMAND-CENTER, FEAT-TASK-BOARD, FEAT-WORK-PACKET-SYSTEM, FEAT-ROLE-MAILBOX, and FEAT-MICRO-TASK-EXECUTOR aligned on view-preset and governed-action language.
- PRIMITIVE_INDEX:
  - Keep the four `v02.170` layout primitives aligned with stable ids and no layout-local authority leaks.
- UI_GUIDANCE:
  - Preserve preset switcher, lane inspector, action preview, inbox triage, and execution queue guidance in Dev Command Center and related surfaces.
- INTERACTION_MATRIX:
  - Keep layout and mailbox-to-execution edges explicit when future passes add new views or action types.

## ACCEPTANCE_CRITERIA (DRAFT)
- Versioned presets exist for board, queue, list, roadmap, inbox triage, and execution queue layouts over the same canonical record family.
- Drag, reorder, quick, and bulk actions show explicit action bindings, target records, approvals, and evidence posture before state changes.
- Layout fallback remains usable when only the base structured-collaboration envelope is available.
- Local-small-model queues remain compact-summary-first and expose mailbox blockers, escalation needs, and validation posture without long-form record ingestion.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on canonical structured collaboration artifacts and schema validation being available first.
- Depends on Dev Command Center typed viewers so presets have a rendering target.
- Blocked on nothing outside the current structured-record and Dev Command Center backlog family.

## RISKS / UNKNOWNs (DRAFT)
- Layout sprawl can recreate hidden state if preset ids and action bindings are not versioned and linted.
- Board metaphors from Jira or other products can overfit software-delivery projects and weaken project-agnostic reuse.
- Queue and board views may diverge unless preset fallbacks and group-by rules are centralized in one registry.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Draft research scouting across BIG_TECH, UNIVERSITY/PAPER, and GITHUB/OSS sources unless the work is strictly internal/mechanical.
- [ ] Draft pillar force multipliers and primitive-matrix combo candidates; create extra stubs instead of guessing.
- [ ] If refinement is likely to grow the primitive index, feature registry, UI guidance, or interaction matrix, treat activation as a spec-version update flow first, then re-activate the WP against the new `SPEC_CURRENT`.
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Dev-Command-Center-Layout-Projection-Registry-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Dev-Command-Center-Layout-Projection-Registry-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
