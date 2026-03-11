# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.
- A stub is the authoritative backlog contract before activation. Task Board, traceability, and Build Order are projections over this stub metadata.
- When a real packet replaces a stub or older packet, the new active packet is whichever file the traceability registry maps for the shared `BASE_WP_ID`.
- For any stub covering current-spec Phase 1 roadmap additions (`[ADD v<current>]`), `ROADMAP_ADD_COVERAGE` MUST enumerate the exact spec line numbers so governance checks can verify no additions were missed.

---

# Work Packet Stub: {{WP_ID}}

## STUB_METADATA
- WP_ID: {{WP_ID}}
- BASE_WP_ID: {{WP_ID}} (stable ID without `-vN`; equals WP_ID for stubs; if WP_ID includes `-vN`, override to the base ID)
- CREATED_AT: {{DATE_ISO}}
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: <pending> (BACKEND | FRONTEND | GOV | CROSS_BOUNDARY)
- BUILD_ORDER_TECH_BLOCKER: <pending> (YES | NO)
- BUILD_ORDER_VALUE_TIER: <pending> (LOW | MEDIUM | HIGH)
- BUILD_ORDER_RISK_TIER: <pending> (LOW | MEDIUM | HIGH)
- BUILD_ORDER_DEPENDS_ON: <pending> (comma-separated Base WP IDs | NONE) (use Base IDs, no `-vN`)
- BUILD_ORDER_BLOCKS: <pending> (comma-separated Base WP IDs | NONE) (use Base IDs, no `-vN`)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: {{ROADMAP_POINTER}}
- ROADMAP_ADD_COVERAGE: SPEC=vXX.XXX; PHASE=7.6.3; LINES={{LINE_NUMBERS_COMMA_SEPARATED}}
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - {{SPEC_ANCHOR_1}}
  - {{SPEC_ANCHOR_2}}

## INTENT (DRAFT)
- What:
- Why:

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - ...
- OUT_OF_SCOPE:
  - ...

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
- RESEARCH_CURRENCY_REQUIRED: <YES|NO>
- TARGET_BUCKETS:
  - BIG_TECH
  - UNIVERSITY|PAPER
  - GITHUB|OSS_DOC
- SEARCH_SEEDS:
  - <fill>
- CANDIDATE_SOURCES:
  - Source: <title> | Kind: <BIG_TECH|UNIVERSITY|PAPER|GITHUB|OSS_DOC> | Date: <YYYY-MM-DD> | Retrieved: <YYYY-MM-DDTHH:MM:SSZ> | URL: <https://...> | Why: <fill>

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
  - PRIM-<fill> (or NONE)
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
- ...

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
- [ ] Create `.GOV/refinements/{{WP_ID}}.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet {{WP_ID}}` (in `.GOV/task_packets/`).
- [ ] Confirm `just create-task-packet {{WP_ID}}` also created `.GOV/roles_shared/WP_COMMUNICATIONS/{{WP_ID}}/`.
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
