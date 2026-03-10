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

# Work Packet Stub: WP-1-Project-Profile-Extension-Registry-v1

## STUB_METADATA
- WP_ID: WP-1-Project-Profile-Extension-Registry-v1
- BASE_WP_ID: WP-1-Project-Profile-Extension-Registry
- CREATED_AT: 2026-03-10T00:55:02.792Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Structured-Collaboration-Schema-Registry
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.168.md 7.6.3 (Phase 1) -> [ADD v02.168] Base structured schema and project-profile contracts
- ROADMAP_ADD_COVERAGE: SPEC=v02.168; PHASE=7.6.3; LINES=46713
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.168.md 2.3.15.5 Base structured schema and project-profile extension contract [ADD v02.168]
  - Handshake_Master_Spec_v02.168.md 10.11 Dev Command Center typed viewers understand base-envelope versus profile-extension fields [ADD v02.168]
  - Handshake_Master_Spec_v02.168.md 2.6.8.10 Role Mailbox export base envelope and profile-extension boundary [ADD v02.168]

## INTENT (DRAFT)
- What: Define and implement the registry of project-profile kinds, extension schema identifiers, compatibility semantics, and generic-viewer fallback rules that sit on top of the shared structured-collaboration base envelope.
- Why: Handshake needs to stop baking repository-governance assumptions into every collaboration artifact while still allowing software-delivery, research, design, and future project kernels to specialize safely.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Registry of supported `project_profile_kind` values and extension schema contracts.
  - Compatibility rules for strict, backward-compatible, and experimental profile extensions.
  - Generic-viewer fallback behavior when an extension is unknown or unavailable.
  - Example extension boundaries for software delivery, research, worldbuilding, and design.
- OUT_OF_SCOPE:
  - Defining the shared base envelope itself.
  - Full implementation of future non-software project kernels.

## UI_UX_SKETCH (DRAFT)
- Principle: prefer enumerating "too many" controls early, consolidate later.
- Include minimalistic in-UI explainers (prefer hover tooltips), and ensure tooltips are accessible (hover + keyboard focus; dismissible; avoid violating WCAG 1.4.13).
- UI_SURFACES:
  - Dev Command Center profile-extension inspector
  - Schema diagnostics drawer
- UI_CONTROLS (buttons/dropdowns/inputs):
  - Control: Profile kind badge | Type: status chip | Tooltip: Shows which project profile kind specialized this record. | Notes: read-only
  - Control: Extension compatibility badge | Type: status chip | Tooltip: Shows whether the extension is strict, backward-compatible, or experimental. | Notes: read-only
- UI_STATES (empty/loading/error):
  - No profile extension
  - Unknown extension
  - Extension incompatible with current viewer
- UI_MICROCOPY_NOTES (labels, helper text, hover explainers):
  - Base envelope only
  - Extension loaded
  - Generic viewer fallback

## RESEARCH_SCOUTING (DRAFT)
- RESEARCH_CURRENCY_REQUIRED: YES
- TARGET_BUCKETS:
  - BIG_TECH
  - UNIVERSITY|PAPER
  - GITHUB|OSS_DOC
- SEARCH_SEEDS:
  - Backstage descriptor metadata and annotations
  - OpenMetadata entity schema specialization
  - GitHub Projects custom fields versus shared item model
  - project-agnostic schema extension patterns
- CANDIDATE_SOURCES:
  - Source: Backstage descriptor format | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-10T01:00:00Z | URL: https://backstage.io/docs/features/software-catalog/descriptor-format | Why: shared envelope plus extensible annotations and kind-specific spec
  - Source: OpenMetadata entity schemas | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-10T01:00:00Z | URL: https://docs.open-metadata.org/v1.8.x/main-concepts/metadata-standard/schemas/entity | Why: type families with reusable common semantics
  - Source: GitHub Projects fields docs | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-10T01:00:00Z | URL: https://docs.github.com/issues/planning-and-tracking-with-projects/understanding-fields/about-fields | Why: shared items plus project-specific field configuration

## RESEARCH_DECISIONS (DRAFT)
- ADOPT:
  - Source: Backstage descriptor format | Pattern: stable common envelope with extension metadata | Why: lets Handshake keep generic parsing while specializing by project profile
- ADAPT:
  - Source: OpenMetadata entity schemas | Pattern: schema families with entity-specific additions | Why: Handshake needs profile-specific fields without redefining the base envelope
- REJECT:
  - Source: repository-specific work-item schemas | Pattern: software-delivery fields mandatory in every record | Why: conflicts with project-agnostic Handshake governance

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
  - PRIM-ProjectProfileExtensionV1
  - PRIM-StructuredCollaborationEnvelopeV1
  - PRIM-StructuredCollaborationSummaryV1
  - PRIM-MirrorSyncState
- PRIMITIVE_MATRIX_COMBO_CANDIDATES:
  - Edge: FEAT-ROLE-MAILBOX -> FEAT-WORK-PACKET-SYSTEM | ROI: H | Effort: M | Notes: shared project-profile boundary across collaboration exports and packet records
  - Edge: FEAT-DEV-COMMAND-CENTER -> FEAT-TASK-BOARD | ROI: M | Effort: M | Notes: generic viewers degrade gracefully when profile extensions are unknown

## FORCE_MULTIPLIER_HYPOTHESES (DRAFT)
- CANDIDATES:
  - Combo: <fill> | Pillars: <comma-separated names|NONE> | Mechanical: <comma-separated engine IDs|NONE> | Primitives/Features: <comma-separated ids|NONE> | Resolution hint: <IN_THIS_STUB|SPIN_OUT_STUB|SPEC_UPDATE> | Notes: <fill>

## EXISTING_CAPABILITY_SCOUTING (DRAFT)
- MATCHED_STUBS:
  - Artifact: WP-1-Structured-Collaboration-Schema-Registry-v1 | Intent: PARTIAL | PrimitiveIndex: PARTIAL | Matrix: PARTIAL | UI: PARTIAL | Resolution hint: KEEP_SEPARATE | Notes: schema registry defines the base envelope; this stub defines specialization and compatibility on top of it
  - Artifact: WP-1-Structured-Collaboration-Artifact-Family-v1 | Intent: PARTIAL | PrimitiveIndex: PARTIAL | Matrix: PARTIAL | UI: NONE | Resolution hint: KEEP_SEPARATE | Notes: file layout and migration direction already exist there
- MATCHED_ACTIVE_OR_DONE_PACKETS:
  - Artifact: <WP-...> | Status: <READY_FOR_DEV|IN_PROGRESS|VALIDATED|OUTDATED_ONLY|BLOCKED|SUPERSEDED> | Intent: <SAME|PARTIAL|DISTINCT> | PrimitiveIndex: <COVERED|MISSING|N/A> | Matrix: <COVERED|MISSING|N/A> | UI: <SAME|PARTIAL|NONE|N/A> | CodeReality: <IMPLEMENTED|PARTIAL|NOT_PRESENT|N/A> | Resolution hint: <REUSE_EXISTING|EXPAND_CURRENT_WP|KEEP_SEPARATE|SPIN_OUT_STUB> | Notes: <fill>
- CODE_REALITY_HINTS:
  - Path: <repo path> | Covers: <primitive|combo|ui-intent|execution> | Notes: <fill>

## APPENDIX_MAINTENANCE_NOTES (DRAFT)
- FEATURE_REGISTRY:
  - FEAT-WORK-PACKET-SYSTEM, FEAT-MICRO-TASK-EXECUTOR, FEAT-TASK-BOARD, FEAT-ROLE-MAILBOX, and FEAT-DEV-COMMAND-CENTER gain project-profile extension notes
- PRIMITIVE_INDEX:
  - Reuse PRIM-ProjectProfileExtensionV1 plus the shared base-envelope primitives
- UI_GUIDANCE:
  - Dev Command Center, Task Board, Work Packet, and Role Mailbox viewers must distinguish base fields from extension fields
- INTERACTION_MATRIX:
  - Add shared-envelope/profile-boundary guidance between Locus Work Tracking, Work Packet System, and Role Mailbox

## ACCEPTANCE_CRITERIA (DRAFT)
- Project-profile kinds and extension-schema identifiers are registered with explicit compatibility semantics.
- Generic viewers can render the base envelope and compact summary when an extension is missing or unknown.
- Repository-specific fields move into software-delivery profile extensions instead of the mandatory base envelope.
- At least one software-delivery extension example and one non-software example are defined without breaking base-envelope validity.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on the shared structured-collaboration schema registry and base-envelope validator contract.
- Future project-kernel refresh, generic viewers, and cross-domain governance remain weaker until this registry exists.

## RISKS / UNKNOWNs (DRAFT)
- Too many profile kinds will turn the registry into a vague dumping ground instead of a stable extension boundary.
- If viewers silently ignore incompatible extensions, operators may think records are complete when they are only partially understood.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Draft research scouting across BIG_TECH, UNIVERSITY/PAPER, and GITHUB/OSS sources unless the work is strictly internal/mechanical.
- [ ] Draft pillar force multipliers and primitive-matrix combo candidates; create extra stubs instead of guessing.
- [ ] If refinement is likely to grow the primitive index, feature registry, UI guidance, or interaction matrix, treat activation as a spec-version update flow first, then re-activate the WP against the new `SPEC_CURRENT`.
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Project-Profile-Extension-Registry-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Project-Profile-Extension-Registry-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
