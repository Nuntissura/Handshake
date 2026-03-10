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

# Work Packet Stub: WP-1-Structured-Collaboration-Artifact-Family-v1

## STUB_METADATA
- WP_ID: WP-1-Structured-Collaboration-Artifact-Family-v1
- BASE_WP_ID: WP-1-Structured-Collaboration-Artifact-Family
- CREATED_AT: 2026-03-10T00:19:00.642Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Locus-Phase1-Integration-Occupancy, WP-1-Role-Mailbox, WP-1-Micro-Task-Executor
- BUILD_ORDER_BLOCKS: WP-1-Markdown-Mirror-Sync-Drift-Guard, WP-1-Dev-Command-Center-Structured-Artifact-Viewer
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.167.md 7.6.3 (Phase 1) -> [ADD v02.167] Canonical structured artifact family
- ROADMAP_ADD_COVERAGE: SPEC=v02.167; PHASE=7.6.3; LINES=46560
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.167.md 2.3.15.5 Structured file projections (portable + model-friendly) [ADD v02.166]
  - Handshake_Master_Spec_v02.167.md 2.3.15.5 Canonical structured collaboration artifact family [ADD v02.167]
  - Handshake_Master_Spec_v02.167.md 7.2 Channel 1: Task Board + Work Packets (Contract Authority) [ADD v02.167]
  - Handshake_Master_Spec_v02.167.md 2.6.8.10 Role Mailbox (Normative) [ADD v02.167]

## INTENT (DRAFT)
- What: Define and implement the canonical structured collaboration artifact family for Work Packets, Micro-Tasks, Task Board projections, and Role Mailbox exports using versioned JavaScript Object Notation or JavaScript Object Notation Lines plus bounded summary records and profile extensions.
- Why: Smaller local models should consume compact structured state without parsing full Markdown packets, and the artifact family must stay reusable across future Handshake project kernels beyond repository-governance-heavy software delivery.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Canonical on-disk layout for `packet.json`, `summary.json`, `index.json`, and `thread.jsonl` artifacts.
  - Versioned schema identifiers and schema-version handling for collaboration artifacts.
  - Project-agnostic base envelope plus profile-extension rules.
  - Compact summary contract for local-small-model ingestion.
  - Validation and migration rules for converting existing Markdown-first records into canonical structured artifacts.
- OUT_OF_SCOPE:
  - Rich Dev Command Center viewer implementation details.
  - Mirror regeneration and drift workflows beyond the interfaces they require.
  - Full project-profile packs for non-software domains.

## UI_UX_SKETCH (DRAFT)
- Principle: prefer enumerating "too many" controls early, consolidate later.
- Include minimalistic in-UI explainers (prefer hover tooltips), and ensure tooltips are accessible (hover + keyboard focus; dismissible; avoid violating WCAG 1.4.13).
- UI_SURFACES:
  - <fill; screens/panels/dialogs/menus>
- UI_CONTROLS (buttons/dropdowns/inputs):
  - Control: <fill> | Type: <fill> | Tooltip: <fill> | Notes: <fill>
- UI_STATES (empty/loading/error):
  - <fill>
- UI_MICROCOPY_NOTES (labels, helper text, hover explainers):
  - <fill>

## RESEARCH_SCOUTING (DRAFT)
- RESEARCH_CURRENCY_REQUIRED: YES
- TARGET_BUCKETS:
  - BIG_TECH
  - UNIVERSITY|PAPER
  - GITHUB|OSS_DOC
- SEARCH_SEEDS:
  - Jira issue field model and board-state projection
  - GitHub Projects fields and view layouts
  - Linear issue fields and workflow-state model
  - project-agnostic structured descriptors and catalog patterns
- CANDIDATE_SOURCES:
  - Source: Atlassian Jira Cloud platform docs | Kind: BIG_TECH | Date: 2025-05-27 | Retrieved: 2026-03-10T00:00:00Z | URL: https://developer.atlassian.com/cloud/jira/platform/rest/v3/api-group-issues/ | Why: field-driven issue authority and typed issue payloads
  - Source: GitHub Projects fields docs | Kind: BIG_TECH | Date: 2025-01-01 | Retrieved: 2026-03-10T00:00:00Z | URL: https://docs.github.com/issues/planning-and-tracking-with-projects/understanding-fields/about-fields | Why: views and planning layouts over typed fields instead of prose boards
  - Source: GitHub Projects roadmap layout docs | Kind: BIG_TECH | Date: 2025-01-01 | Retrieved: 2026-03-10T00:00:00Z | URL: https://docs.github.com/issues/planning-and-tracking-with-projects/customizing-views-in-your-project/customizing-the-roadmap-layout | Why: derived roadmap/board layouts as consumers of the same record set
  - Source: Linear API docs | Kind: BIG_TECH | Date: 2025-01-01 | Retrieved: 2026-03-10T00:00:00Z | URL: https://developers.linear.app/docs/graphql/working-with-the-graphql-api | Why: compact issue/state model for low-overhead structured work items

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
- A versioned structured file family is defined for Work Packets, Micro-Tasks, Task Board projections, and Role Mailbox exports.
- Every canonical artifact exposes a compact summary usable by local small models without loading long Markdown by default.
- Base-envelope fields are project-agnostic and profile extensions are explicitly separated.
- Migration guidance exists for Markdown-first artifacts to become mirrors or sidecars instead of the only machine-readable authority.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on existing Locus storage and tracking primitives.
- Depends on current Role Mailbox message/export contracts.
- Should stay aligned with future project-kernel generalization so repository-specific assumptions do not harden further.

## RISKS / UNKNOWNs (DRAFT)
- Risk: overfitting the base schema to repository-centric software delivery. Mitigation: keep project-specific data in profile extensions.
- Risk: summary records drift from canonical packet records. Mitigation: derive summaries mechanically and validate hash or update-time linkage.
- Risk: premature migration complexity. Mitigation: keep the pass hybrid and Phase 1 focused rather than forcing a big-bang replacement.

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
- [ ] Create `.GOV/refinements/WP-1-Structured-Collaboration-Artifact-Family-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Structured-Collaboration-Artifact-Family-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
