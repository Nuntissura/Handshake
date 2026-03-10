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

# Work Packet Stub: WP-1-Structured-Collaboration-Schema-Registry-v1

## STUB_METADATA
- WP_ID: WP-1-Structured-Collaboration-Schema-Registry-v1
- BASE_WP_ID: WP-1-Structured-Collaboration-Schema-Registry
- CREATED_AT: 2026-03-10T00:55:02.792Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Structured-Collaboration-Artifact-Family
- BUILD_ORDER_BLOCKS: WP-1-Project-Profile-Extension-Registry, WP-1-Markdown-Mirror-Sync-Drift-Guard, WP-1-Dev-Command-Center-Structured-Artifact-Viewer
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.168.md 7.6.3 (Phase 1) -> [ADD v02.168] Base structured schema and project-profile contracts
- ROADMAP_ADD_COVERAGE: SPEC=v02.168; PHASE=7.6.3; LINES=46713
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.168.md 2.3.15.2 shared structured-collaboration envelope fields inside TrackedWorkPacket and TrackedMicroTask [ADD v02.168]
  - Handshake_Master_Spec_v02.168.md 2.3.15.5 Base structured schema and project-profile extension contract [ADD v02.168]
  - Handshake_Master_Spec_v02.168.md 2.3.15.5 Compact summary contract for local small models [ADD v02.168]
  - Handshake_Master_Spec_v02.168.md 2.6.8.10 Repo export RoleMailboxIndexV1 / RoleMailboxThreadLineV1 base envelope [ADD v02.168]

## INTENT (DRAFT)
- What: Define and implement the canonical schema registry, validators, compatibility rules, and summary-schema checks for the shared structured-collaboration envelope used by Work Packets, Micro-Tasks, Task Board projections, and Role Mailbox exports.
- Why: Handshake now requires one reusable base envelope and compact summary contract. Without a registry and deterministic validation path, each implementation surface will drift into slightly different field names, versions, and parser assumptions.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Canonical schema identifiers and versions for the shared base envelope and summary contract.
  - Validation rules for `packet.json`, `summary.json`, `index.json`, and `thread.jsonl` collaboration artifacts.
  - Compatibility and migration policy for base-envelope changes and backward-compatible readers.
  - Deterministic validation outputs consumable by Dev Command Center and governance gates.
- OUT_OF_SCOPE:
  - Rich Dev Command Center rendering logic beyond the validation data it needs.
  - Project-specific extension payload definitions beyond the hooks they must satisfy.

## UI_UX_SKETCH (DRAFT)
- Principle: prefer enumerating "too many" controls early, consolidate later.
- Include minimalistic in-UI explainers (prefer hover tooltips), and ensure tooltips are accessible (hover + keyboard focus; dismissible; avoid violating WCAG 1.4.13).
- UI_SURFACES:
  - Dev Command Center schema diagnostics panel
  - Raw-record inspector validation banner
- UI_CONTROLS (buttons/dropdowns/inputs):
  - Control: Schema version badge | Type: status chip | Tooltip: Shows which canonical schema version this record matches. | Notes: read-only
  - Control: Validation detail drawer | Type: drawer toggle | Tooltip: Open base-envelope and summary validation results. | Notes: advanced operator view
- UI_STATES (empty/loading/error):
  - Unknown schema version
  - Base-envelope invalid
  - Summary mismatch
- UI_MICROCOPY_NOTES (labels, helper text, hover explainers):
  - Base envelope valid
  - Profile extension skipped
  - Summary drift detected

## RESEARCH_SCOUTING (DRAFT)
- RESEARCH_CURRENCY_REQUIRED: YES
- TARGET_BUCKETS:
  - BIG_TECH
  - UNIVERSITY|PAPER
  - GITHUB|OSS_DOC
- SEARCH_SEEDS:
  - Atlassian issue field schema and board projection
  - GitHub Projects field model and derived views
  - Backstage descriptor format shared envelope
  - OpenMetadata entity schema layering
- CANDIDATE_SOURCES:
  - Source: Atlassian Jira Cloud issue APIs | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-10T01:00:00Z | URL: https://developer.atlassian.com/cloud/jira/platform/rest/v3/api-group-issues/ | Why: typed issue authority and stable field envelopes
  - Source: GitHub Projects fields docs | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-10T01:00:00Z | URL: https://docs.github.com/issues/planning-and-tracking-with-projects/understanding-fields/about-fields | Why: field-driven items with multiple derived views
  - Source: Backstage descriptor format | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-10T01:00:00Z | URL: https://backstage.io/docs/features/software-catalog/descriptor-format | Why: shared envelope plus kind-specific extensibility
  - Source: OpenMetadata entity schemas | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-10T01:00:00Z | URL: https://docs.open-metadata.org/v1.8.x/main-concepts/metadata-standard/schemas/entity | Why: schema family layering with typed entity records

## RESEARCH_DECISIONS (DRAFT)
- ADOPT:
  - Source: Backstage descriptor format | Pattern: common envelope plus kind-specific body | Why: aligns with Handshake base-envelope plus profile-extension split
- ADAPT:
  - Source: GitHub Projects fields docs | Pattern: views derived from stable item fields | Why: Handshake should keep board and queue projections downstream of the same canonical records
- REJECT:
  - Source: Atlassian Jira board posture | Pattern: board-column configuration treated as authoritative workflow state | Why: Handshake wants field authority in canonical records, not board layout

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
  - PRIM-StructuredCollaborationEnvelopeV1
  - PRIM-StructuredCollaborationSummaryV1
  - PRIM-ProjectProfileExtensionV1
  - PRIM-MirrorSyncState
- PRIMITIVE_MATRIX_COMBO_CANDIDATES:
  - Edge: FEAT-LOCUS-WORK-TRACKING -> FEAT-WORK-PACKET-SYSTEM | ROI: H | Effort: M | Notes: shared base envelope and compact summary registry
  - Edge: FEAT-DEV-COMMAND-CENTER -> FEAT-ROLE-MAILBOX | ROI: M | Effort: M | Notes: schema-aware triage over base envelope plus mailbox extension fields

## FORCE_MULTIPLIER_HYPOTHESES (DRAFT)
- CANDIDATES:
  - Combo: <fill> | Pillars: <comma-separated names|NONE> | Mechanical: <comma-separated engine IDs|NONE> | Primitives/Features: <comma-separated ids|NONE> | Resolution hint: <IN_THIS_STUB|SPIN_OUT_STUB|SPEC_UPDATE> | Notes: <fill>

## EXISTING_CAPABILITY_SCOUTING (DRAFT)
- MATCHED_STUBS:
  - Artifact: WP-1-Structured-Collaboration-Artifact-Family-v1 | Intent: PARTIAL | PrimitiveIndex: PARTIAL | Matrix: PARTIAL | UI: NONE | Resolution hint: KEEP_SEPARATE | Notes: owns canonical file family and migration direction, but not the reusable schema registry and validator contract
  - Artifact: WP-1-Dev-Command-Center-Structured-Artifact-Viewer-v1 | Intent: PARTIAL | PrimitiveIndex: PARTIAL | Matrix: PARTIAL | UI: PARTIAL | Resolution hint: KEEP_SEPARATE | Notes: viewer depends on this stub but should not define schema authority
- MATCHED_ACTIVE_OR_DONE_PACKETS:
  - Artifact: <WP-...> | Status: <READY_FOR_DEV|IN_PROGRESS|VALIDATED|OUTDATED_ONLY|BLOCKED|SUPERSEDED> | Intent: <SAME|PARTIAL|DISTINCT> | PrimitiveIndex: <COVERED|MISSING|N/A> | Matrix: <COVERED|MISSING|N/A> | UI: <SAME|PARTIAL|NONE|N/A> | CodeReality: <IMPLEMENTED|PARTIAL|NOT_PRESENT|N/A> | Resolution hint: <REUSE_EXISTING|EXPAND_CURRENT_WP|KEEP_SEPARATE|SPIN_OUT_STUB> | Notes: <fill>
- CODE_REALITY_HINTS:
  - Path: <repo path> | Covers: <primitive|combo|ui-intent|execution> | Notes: <fill>

## APPENDIX_MAINTENANCE_NOTES (DRAFT)
- FEATURE_REGISTRY:
  - FEAT-LOCUS-WORK-TRACKING, FEAT-MICRO-TASK-EXECUTOR, FEAT-ROLE-MAILBOX, FEAT-TASK-BOARD, FEAT-WORK-PACKET-SYSTEM, and FEAT-DEV-COMMAND-CENTER gain base-envelope ownership notes
- PRIMITIVE_INDEX:
  - Add PRIM-StructuredCollaborationEnvelopeV1, PRIM-StructuredCollaborationSummaryV1, PRIM-ProjectProfileExtensionV1, and PRIM-MirrorSyncState
- UI_GUIDANCE:
  - Dev Command Center, Role Mailbox, Task Board, and Work Packet viewers gain schema-validation and base-envelope degradation expectations
- INTERACTION_MATRIX:
  - Add Locus Work Tracking -> Work Packet System shared-envelope interaction guidance

## ACCEPTANCE_CRITERIA (DRAFT)
- A canonical schema registry exists for the shared structured-collaboration envelope and compact summary contract.
- Work Packet, Micro-Task, Task Board projection, and Role Mailbox export validators reject missing required base-envelope fields deterministically.
- Unknown or incompatible schema versions surface machine-readable validation results instead of silent fallback.
- Dev Command Center and local-small-model ingestion can rely on the compact summary contract without record-shape guessing.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on the canonical structured collaboration artifact family and its file layout contract.
- Blocks reliable project-profile extension validation, mirror-sync drift handling, and generic Dev Command Center structured viewers.

## RISKS / UNKNOWNs (DRAFT)
- If the base envelope is too repository-specific, the later governance-kernel refresh will need a breaking rewrite.
- If summary schemas drift from detail schemas, smaller local models will get cheap but misleading state.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Draft research scouting across BIG_TECH, UNIVERSITY/PAPER, and GITHUB/OSS sources unless the work is strictly internal/mechanical.
- [ ] Draft pillar force multipliers and primitive-matrix combo candidates; create extra stubs instead of guessing.
- [ ] If refinement is likely to grow the primitive index, feature registry, UI guidance, or interaction matrix, treat activation as a spec-version update flow first, then re-activate the WP against the new `SPEC_CURRENT`.
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Structured-Collaboration-Schema-Registry-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Structured-Collaboration-Schema-Registry-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
