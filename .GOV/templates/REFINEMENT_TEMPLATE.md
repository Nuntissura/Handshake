## TECHNICAL_REFINEMENT (MASTER SPEC)

Requirements (HARD):
- This block is REQUIRED for any packet with Status: Ready for Dev / In Progress.
- For EACH SPEC_ANCHOR, include an excerpt window (start/end lines) AND a context token that must appear within that window in the current SPEC_TARGET_RESOLVED spec file.
- Matching rule: context match only (token-in-window), not exact content match.
- Even when ENRICHMENT_NEEDED=NO, you MUST include REASON_NO_ENRICHMENT and SPEC_EXCERPTS for every anchor.
- If ENRICHMENT_NEEDED=YES, you MUST include the full Proposed Spec Enrichment text (verbatim Markdown) that could be copy-pasted into the Master Spec.
- Keep this file ASCII-only. Non-ASCII characters must be written as \\uXXXX escapes inside the excerpt block.
- This file is the Technical Refinement Block required by ORCHESTRATOR_PROTOCOL Part 2.5.2.

### METADATA
- WP_ID: {{WP_ID}}
- REFINEMENT_FORMAT_VERSION: 2026-03-06
- CREATED_AT: {{DATE_ISO}}
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> {{SPEC_TARGET_RESOLVED}}
- SPEC_TARGET_SHA1: {{SPEC_TARGET_SHA1}}
- USER_REVIEW_STATUS: PENDING
- USER_SIGNATURE: <pending>
- USER_APPROVAL_EVIDENCE: <pending> (must equal: APPROVE REFINEMENT {{WP_ID}})
- STUB_WP_IDS: <pending> (comma-separated WP-... IDs | NONE)

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- <fill; write NONE if no gaps>

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: <fill; e.g., 30m|2h|4h>
- SEARCH_SCOPE: <fill; key terms + where you searched>
- REFERENCES: <fill; include vendor docs, papers, and OSS repos; write NONE + reason only if truly not applicable>
- PATTERNS_EXTRACTED: <fill; what to steal (constraints/invariants/interfaces)>
- DECISIONS (ADOPT/ADAPT/REJECT): <fill; include rationale>
- LICENSE/IP_NOTES: <fill; include any constraints for code-level reuse>
- SPEC_IMPACT: PENDING (YES | NO)
- SPEC_IMPACT_REASON: <fill; if YES, point to Main Body and/or EOF Appendix blocks that must change>

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- <fill; write NONE if not applicable>

### RED_TEAM_ADVISORY (security failure modes)
- <fill; write NONE if not applicable>

### PRIMITIVES (traits/structs/enums)
- PRIMITIVES_TOUCHED (IDs):
  - PRIM-<fill>
- PRIMITIVES_NEW_OR_UPDATED (IDs):
  - PRIM-<fill> (or NONE)
- NOTES:
  - <fill>

### PRIMITIVE_INDEX (Appendix 12.4: HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX)
- PRIMITIVE_INDEX_ACTION: PENDING (UPDATED | NO_CHANGE)
- PRIMITIVE_INDEX_REASON_NO_CHANGE: <fill if PRIMITIVE_INDEX_ACTION=NO_CHANGE>
- PRIMITIVE_INDEX_UPDATE_NOTES:
  - <fill>

### PILLAR_ALIGNMENT (Handshake pillars cross-check)
- Rule: Refinement MUST explicitly consider pillar alignment and interconnections (force multipliers). If unknown, write UNKNOWN and create stubs instead of guessing.
- Required rubric lines (one per pillar; do not delete lines, fill values):
  - PILLAR: Flight Recorder | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - PILLAR: Calendar | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - PILLAR: Monaco | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - PILLAR: Word clone | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - PILLAR: Excel clone | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - PILLAR: Locus | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - PILLAR: Loom | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - PILLAR: Work packets (product, not repo) | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - PILLAR: Task board (product, not repo) | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - PILLAR: MicroTask | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - PILLAR: Command Center | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - PILLAR: Spec to prompt | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - PILLAR: SQL to PostgreSQL shift readiness | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - PILLAR: LLM-friendly data | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - PILLAR: Stage | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - PILLAR: Studio | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - PILLAR: Atelier/Lens | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - PILLAR: Skill distillation / LoRA | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - PILLAR: ACE | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - PILLAR: RAG | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
- PILLAR_ALIGNMENT_VERDICT: PENDING (OK | NEEDS_SPEC_UPDATE | NEEDS_STUBS)

### PRIMITIVE_MATRIX (high-ROI combos; cross-primitive / cross-feature)
- MATRIX_SCAN_TIMEBOX: <fill; e.g., 30m|2h|4h>
- MATRIX_SCAN_NOTES:
  - <fill; include local+cloud model compatibility and tool mixing opportunities>
- IMX_EDGE_IDS_ADDED_OR_UPDATED: <pending> (comma-separated IMX-... IDs | NONE)
- Candidate interaction edges to add/update in Spec Appendix 12.6 (HS-APPX-INTERACTION-MATRIX):
  - Edge: <from_kind/from_id> -> <to_kind/to_id>
  - Kind: <fill>
  - ROI: <HIGH|MEDIUM|LOW>
  - Effort: <HIGH|MEDIUM|LOW>
  - Spec refs: <fill>
  - In-scope for this WP: PENDING (YES | NO)
  - If NO: create a stub WP and record it in TASK_BOARD Stub Backlog (order is not priority).
- PRIMITIVE_MATRIX_VERDICT: PENDING (OK | NEEDS_STUBS | NONE_FOUND)
- PRIMITIVE_MATRIX_REASON: <fill>

### UI_UX_RUBRIC (early UI/UX thinking; prefer too many controls early)
- UI_UX_APPLICABLE: PENDING (YES | NO)
- UI_UX_REASON_NO: <fill if UI_UX_APPLICABLE=NO>
- UI_SURFACES:
  - <fill; screens/panels/dialogs/menus>
- UI_CONTROLS (buttons/dropdowns/inputs):
  - Control: <fill> | Type: <fill> | Tooltip: <fill> | Notes: <fill>
- UI_STATES (empty/loading/error):
  - <fill>
- UI_MICROCOPY_NOTES (labels, helper text, hover explainers):
  - <fill>
- UI_ACCESSIBILITY_NOTES:
  - Tooltips must work on hover and keyboard focus; be dismissible; do not obscure content (WCAG 1.4.13).
- UI_UX_VERDICT: PENDING (OK | NEEDS_STUBS | UNKNOWN)

### ROADMAP_PHASE_SPLIT (only if scope must be phased)
- PHASE_SPLIT_NEEDED: PENDING (YES | NO)
- If YES: update the Roadmap (Spec 7.6) using the fixed per-phase fields below (do not invent new per-phase block types).
- Per phase, include exactly:
  - Goal:
  - MUST deliver:
  - Key risks addressed in Phase n:
  - Acceptance criteria:
  - Explicitly OUT of scope:
  - Mechanical Track:
  - Atelier Track:
  - Distillation Track:
  - Vertical slice:

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [ ] PASS
- Explicitly named: [ ] PASS
- Specific: [ ] PASS
- Measurable acceptance criteria: [ ] PASS
- No ambiguity: [ ] PASS
- CLEARLY_COVERS_VERDICT: PENDING
- CLEARLY_COVERS_REASON: <fill>
- AMBIGUITY_FOUND: PENDING (YES | NO)
- AMBIGUITY_REASON: <fill; write NONE if AMBIGUITY_FOUND=NO>

### ENRICHMENT
- ENRICHMENT_NEEDED: PENDING
- REASON_NO_ENRICHMENT: <fill if ENRICHMENT_NEEDED=NO>

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: <fill (example: Handshake_Master_Spec_v02.99.md 2.3.12.5 [CX-DBP-030])>
- CONTEXT_START_LINE: <fill integer>
- CONTEXT_END_LINE: <fill integer>
- CONTEXT_TOKEN: <fill exact string that must appear between start/end lines in SPEC_TARGET_RESOLVED>
- EXCERPT_ASCII_ESCAPED:
  ```text
  <paste the relevant excerpt; ASCII-only; use \\uXXXX escapes when needed>
  ```

#### ANCHOR 2
- SPEC_ANCHOR: <fill>
- CONTEXT_START_LINE: <fill integer>
- CONTEXT_END_LINE: <fill integer>
- CONTEXT_TOKEN: <fill>
- EXCERPT_ASCII_ESCAPED:
  ```text
  <paste excerpt>
  ```
