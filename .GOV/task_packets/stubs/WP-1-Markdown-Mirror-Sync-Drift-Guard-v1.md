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

# Work Packet Stub: WP-1-Markdown-Mirror-Sync-Drift-Guard-v1

## STUB_METADATA
- WP_ID: WP-1-Markdown-Mirror-Sync-Drift-Guard-v1
- BASE_WP_ID: WP-1-Markdown-Mirror-Sync-Drift-Guard
- CREATED_AT: 2026-03-10T00:19:00.642Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: MEDIUM
- BUILD_ORDER_DEPENDS_ON: WP-1-Structured-Collaboration-Artifact-Family, WP-1-Locus-Phase1-Integration-Occupancy
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.167.md 7.6.3 (Phase 1) -> [ADD v02.167] Canonical structured artifact family
- ROADMAP_ADD_COVERAGE: SPEC=v02.167; PHASE=7.6.3; LINES=46560
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.167.md 2.3.15.5 Structured file projections (portable + model-friendly) [ADD v02.166]
  - Handshake_Master_Spec_v02.167.md 2.3.15.5 Canonical structured collaboration artifact family [ADD v02.167]
  - Handshake_Master_Spec_v02.167.md 10.11.5.18 Projected Boards and Queue Layouts over Structured Records [ADD v02.167]

## INTENT (DRAFT)
- What: Implement deterministic Markdown mirror generation, drift detection, advisory-edit handling, and reconciliation status for structured collaboration artifacts.
- Why: The product can keep readable Markdown without letting mirrors silently become the authority or diverge from the canonical JavaScript Object Notation records.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Mirror regeneration rules for Work Packets and Task Board outputs.
  - Drift detection states such as synchronized, stale, advisory edit, and normalization required.
  - Operator-visible evidence for why a mirror differs from its canonical record.
  - Safe handling of manual Markdown edits without silent overwrite.
- OUT_OF_SCOPE:
  - Designing the canonical structured artifact schemas themselves.
  - Rich board or typed-viewer presentation beyond the states they need to show.

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
  - PRIM-TrackedWorkPacket
  - PRIM-TaskBoardEntry
  - PRIM-SpecSessionLogEntry
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
- Markdown mirrors can be regenerated deterministically from canonical structured records.
- Drift states are explicit and machine-readable.
- Manual Markdown edits are surfaced as advisory until normalized back into canonical records.
- Dev Command Center and planning views can distinguish synchronized versus stale mirrors without reading raw diffs first.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on the structured collaboration artifact family.
- Depends on Task Board and Work Packet identifiers being stable across regeneration.

## RISKS / UNKNOWNs (DRAFT)
- Risk: mirror generation clobbers useful human notes. Mitigation: keep note sidecars append-only and separate from generated mirrors.
- Risk: drift signaling is too noisy. Mitigation: define bounded statuses and normalization paths instead of raw diff spam.

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
- [ ] Create `.GOV/refinements/WP-1-Markdown-Mirror-Sync-Drift-Guard-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Markdown-Mirror-Sync-Drift-Guard-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
