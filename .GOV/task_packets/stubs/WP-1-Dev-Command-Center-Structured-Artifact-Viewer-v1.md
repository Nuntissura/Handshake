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
- ROADMAP_POINTER: Handshake_Master_Spec_v02.167.md 7.6.3 (Phase 1) -> [ADD v02.167] Canonical structured artifact family
- ROADMAP_ADD_COVERAGE: SPEC=v02.167; PHASE=7.6.3; LINES=46560
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.167.md 10.11.5.17 Structured Work Records, Notes, and Collaboration Inbox [ADD v02.166]
  - Handshake_Master_Spec_v02.167.md 10.11.5.18 Projected Boards and Queue Layouts over Structured Records [ADD v02.167]

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
  - <fill>

## RESEARCH_SCOUTING (DRAFT)
- RESEARCH_CURRENCY_REQUIRED: YES
- TARGET_BUCKETS:
  - BIG_TECH
  - UNIVERSITY|PAPER
  - GITHUB|OSS_DOC
- SEARCH_SEEDS:
  - Jira board columns and issue field projections
  - GitHub Projects fields and views
  - Linear issue lists and state filters
- CANDIDATE_SOURCES:
  - Source: Atlassian support docs for Kanban boards | Kind: BIG_TECH | Date: 2025-01-01 | Retrieved: 2026-03-10T00:00:00Z | URL: https://support.atlassian.com/jira-software-cloud/docs/configure-columns/ | Why: board layout as a mapping over issue state
  - Source: GitHub Projects fields docs | Kind: BIG_TECH | Date: 2025-01-01 | Retrieved: 2026-03-10T00:00:00Z | URL: https://docs.github.com/issues/planning-and-tracking-with-projects/understanding-fields/about-fields | Why: field-driven projections and multiple views over the same items
  - Source: GitHub roadmap layout docs | Kind: BIG_TECH | Date: 2025-01-01 | Retrieved: 2026-03-10T00:00:00Z | URL: https://docs.github.com/issues/planning-and-tracking-with-projects/customizing-views-in-your-project/customizing-the-roadmap-layout | Why: roadmap as a layout, not a second issue authority

## RESEARCH_DECISIONS (DRAFT)
- ADOPT:
  - Source: <title> | Pattern: <fill> | Why: <fill>
- ADAPT:
  - Source: <title> | Pattern: <fill> | Why: <fill>
- REJECT:
  - Source: <title> | Pattern: <fill> | Why: <fill>

## GITHUB_PROJECT_SCOUTING (DRAFT)
- SEARCH_QUERIES:
  - <fill>
- MATCHED_PROJECTS:
  - Repo: <owner/name> | Intent: <SAME|ADJACENT|IMPLEMENTATION|UI_PATTERN|ARCH_PATTERN> | Decision hint: <ADOPT|ADAPT|REJECT|TRACK_ONLY> | Impact hint: <NONE|EXPAND_SCOPE|NEW_STUB|SPEC_UPDATE|UI_ENRICHMENT> | Notes: <fill>

## PILLAR_FORCE_MULTIPLIERS (DRAFT)
- TOUCHED_OR_UNKNOWN_PILLARS:
  - PILLAR: <fill> | STATUS: <TOUCHED|UNKNOWN> | NOTES: <fill> | Stub follow-up: <THIS_STUB|WP-...|NONE>

## MECHANICAL_ENGINE_SCOUTING (DRAFT)
- TOUCHED_OR_UNKNOWN_ENGINES:
  - ENGINE: <title> | ENGINE_ID: <engine.id> | STATUS: <TOUCHED|UNKNOWN> | NOTES: <fill> | Stub follow-up: <THIS_STUB|WP-...|NONE>

## PRIMITIVES_AND_MATRIX_NOTES (DRAFT)
- PRIMITIVES_TOUCHED:
  - PRIM-TrackedWorkPacket
  - PRIM-TrackedMicroTask
  - PRIM-TaskBoardEntry
  - PRIM-RoleMailboxBodyV0_5
- PRIMITIVE_MATRIX_COMBO_CANDIDATES:
  - Edge: <from_kind/from_id> -> <to_kind/to_id> | ROI: <H|M|L> | Effort: <H|M|L> | Notes: <fill>

## FORCE_MULTIPLIER_HYPOTHESES (DRAFT)
- CANDIDATES:
  - Combo: <fill> | Pillars: <comma-separated names|NONE> | Mechanical: <comma-separated engine IDs|NONE> | Primitives/Features: <comma-separated ids|NONE> | Resolution hint: <IN_THIS_STUB|SPIN_OUT_STUB|SPEC_UPDATE> | Notes: <fill>

## EXISTING_CAPABILITY_SCOUTING (DRAFT)
- MATCHED_STUBS:
  - Artifact: <WP-...> | Intent: <SAME|PARTIAL|DISTINCT> | PrimitiveIndex: <COVERED|MISSING|N/A> | Matrix: <COVERED|MISSING|N/A> | UI: <SAME|PARTIAL|NONE|N/A> | Resolution hint: <REUSE_EXISTING|EXPAND_THIS_STUB|KEEP_SEPARATE> | Notes: <fill>
- MATCHED_ACTIVE_OR_DONE_PACKETS:
  - Artifact: <WP-...> | Status: <READY_FOR_DEV|IN_PROGRESS|VALIDATED|OUTDATED_ONLY|BLOCKED|SUPERSEDED> | Intent: <SAME|PARTIAL|DISTINCT> | PrimitiveIndex: <COVERED|MISSING|N/A> | Matrix: <COVERED|MISSING|N/A> | UI: <SAME|PARTIAL|NONE|N/A> | CodeReality: <IMPLEMENTED|PARTIAL|NOT_PRESENT|N/A> | Resolution hint: <REUSE_EXISTING|EXPAND_CURRENT_WP|KEEP_SEPARATE|SPIN_OUT_STUB> | Notes: <fill>
- CODE_REALITY_HINTS:
  - Path: <repo path> | Covers: <primitive|combo|ui-intent|execution> | Notes: <fill>

## APPENDIX_MAINTENANCE_NOTES (DRAFT)
- FEATURE_REGISTRY:
  - <fill>
- PRIMITIVE_INDEX:
  - <fill>
- UI_GUIDANCE:
  - <fill>
- INTERACTION_MATRIX:
  - <fill>

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

## DEPENDENCIES / BLOCKERS (DRAFT)
- ...

## RISKS / UNKNOWNs (DRAFT)
- ...

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
