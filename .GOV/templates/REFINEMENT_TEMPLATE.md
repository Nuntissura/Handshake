## TECHNICAL_REFINEMENT (MASTER SPEC)

Requirements (HARD):
- This block is REQUIRED for any packet with Status: Ready for Dev / In Progress.
- For EACH SPEC_ANCHOR, include an excerpt window (start/end lines) AND a context token that must appear within that window in the current SPEC_TARGET_RESOLVED spec file.
- Matching rule: context match only (token-in-window), not exact content match.
- Even when ENRICHMENT_NEEDED=NO, you MUST include REASON_NO_ENRICHMENT and SPEC_EXCERPTS for every anchor.
- If ENRICHMENT_NEEDED=YES, you MUST include the full Proposed Spec Enrichment text (verbatim Markdown) that could be copy-pasted into the Master Spec.
- Keep this file ASCII-only. Non-ASCII characters must be written as \\uXXXX escapes inside the excerpt block.
- This file is the Technical Refinement Block required by the current ORCHESTRATOR_PROTOCOL refinement workflow.

### METADATA
- WP_ID: {{WP_ID}}
- REFINEMENT_FORMAT_VERSION: 2026-03-16
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- CREATED_AT: {{DATE_ISO}}
- SPEC_TARGET_RESOLVED: .GOV/spec/SPEC_CURRENT.md -> {{SPEC_TARGET_RESOLVED}}
- SPEC_TARGET_SHA1: {{SPEC_TARGET_SHA1}}
- USER_REVIEW_STATUS: PENDING
- USER_SIGNATURE: <pending>
- USER_APPROVAL_EVIDENCE: <pending> (must equal: APPROVE REFINEMENT {{WP_ID}})
- STUB_WP_IDS: <pending> (comma-separated WP-... IDs | NONE)

### REQUIRED SECTIONS (per the current ORCHESTRATOR_PROTOCOL refinement workflow)

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

### RESEARCH_CURRENCY (current external signal scan; mandatory unless the WP is strictly internal/mechanical)
- Rule: if the WP is an internal repo-governed change or product-governance mirror patch already grounded in the current Master Spec plus local code/runtime truth, it is valid and often preferable to set `RESEARCH_CURRENCY_REQUIRED=NO`. Do not force unrelated or generic web research just to populate this section.
- RESEARCH_CURRENCY_REQUIRED: PENDING (YES | NO)
- RESEARCH_CURRENCY_REASON_NO: <fill if RESEARCH_CURRENCY_REQUIRED=NO>
- SOURCE_MAX_AGE_DAYS: <fill integer 30-730; if RESEARCH_CURRENCY_REQUIRED=NO write N/A>
- SOURCE_LOG:
  - Source: <title> | Kind: <BIG_TECH|UNIVERSITY|PAPER|GITHUB|OSS_DOC> | Date: <YYYY-MM-DD> | Retrieved: <YYYY-MM-DDTHH:MM:SSZ> | URL: <https://...> | Why: <fill>
- RESEARCH_SYNTHESIS:
  - <fill; what improves Handshake or what to avoid>
- RESEARCH_GAPS_TO_TRACK:
  - <fill; write NONE if none>
- RESEARCH_CURRENCY_VERDICT: PENDING (CURRENT | STALE | NOT_APPLICABLE)

### RESEARCH_DEPTH (prevent shallow source logging)
- Rule: if RESEARCH_CURRENCY_REQUIRED=YES, refinement must record at least one adopted pattern, one adapted pattern, and one rejected pattern grounded in the SOURCE_LOG. Do not only list sources; prove how they changed the WP.
- ADOPT_PATTERNS:
  - Source: <title from SOURCE_LOG> | Pattern: <fill> | Why: <fill>
- ADAPT_PATTERNS:
  - Source: <title from SOURCE_LOG> | Pattern: <fill> | Why: <fill>
- REJECT_PATTERNS:
  - Source: <title from SOURCE_LOG> | Pattern: <fill> | Why: <fill>
- RESEARCH_DEPTH_VERDICT: PENDING (PASS | NOT_APPLICABLE)

### GITHUB_PROJECT_SCOUTING (same-topic repo exploration; feed useful findings back into governance)
- Rule: if RESEARCH_CURRENCY_REQUIRED=YES, inspect topic-adjacent GitHub projects/repos that touch the same intent, implementation topic, or UI surface. This is for discovering better execution patterns, richer feature combinations, and UI/UX force multipliers. Useful findings MUST flow back into spec/governance through scope expansion, new stubs, spec updates, or UI enrichment. If no directly topical project search exists, mark this section `NOT_APPLICABLE`; do not substitute off-topic searches.
- SEARCH_QUERIES:
  - <fill; repo/topic search query or angle>
- MATCHED_PROJECTS:
  - Source: <title from SOURCE_LOG with Kind: GITHUB> | Repo: <owner/name> | URL: <https://github.com/owner/name> | Intent: <SAME|ADJACENT|IMPLEMENTATION|UI_PATTERN|ARCH_PATTERN> | Decision: <ADOPT|ADAPT|REJECT|TRACK_ONLY> | Impact: <NONE|EXPAND_SCOPE|NEW_STUB|SPEC_UPDATE_NOW|UI_ENRICHMENT> | Stub: <WP-... | NONE> | Notes: <fill>
- GITHUB_PROJECT_SCOUTING_VERDICT: PENDING (PASS | NOT_APPLICABLE)

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- <fill; write NONE if not applicable>

### RED_TEAM_ADVISORY (security failure modes)
- <fill; write NONE if not applicable>

### PRIMITIVES (traits/structs/enums)
- PRIMITIVES_TOUCHED (IDs):
  - PRIM-<fill>
- PRIMITIVES_EXPOSED (IDs):
  - PRIM-<fill> (or NONE)
- PRIMITIVES_CREATED (IDs):
  - PRIM-<fill> (or NONE)
- PRIMITIVES_NEW_OR_UPDATED (IDs):
  - PRIM-<fill> (or NONE)
- NOTES:
  - <fill>

### PRIMITIVE_INDEX (Appendix 12.4: HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX)
- PRIMITIVE_INDEX_ACTION: PENDING (UPDATED | NO_CHANGE)
- PRIMITIVE_INDEX_REASON_NO_CHANGE: <fill if PRIMITIVE_INDEX_ACTION=NO_CHANGE>
- PRIMITIVE_INDEX_UPDATE_NOTES:
  - <fill>
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_DISCOVERED: <comma-separated PRIM-... IDs | NONE>
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_RESOLUTION: PENDING (ATTACHED | STUBBED | MIXED | NONE)
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_ATTACHED_THIS_PASS: <comma-separated PRIM-... IDs | NONE>
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_STUB_WP_IDS: <comma-separated WP-... IDs | NONE>
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_REASON: <fill>

### APPENDIX_MAINTENANCE (spec appendix follow-through)
- Rule: if any appendix action below is `UPDATED`, this refinement is declaring a Master Spec version bump. In that case set `APPENDIX_MAINTENANCE_VERDICT=NEEDS_SPEC_UPDATE`, set `SPEC_IMPACT=YES`, set `ENRICHMENT_NEEDED=YES`, and include the verbatim appendix update text in `PROPOSED_SPEC_ENRICHMENT`. Packet creation stays blocked until the new spec version exists and `SPEC_CURRENT` is advanced.
- FEATURE_REGISTRY_ACTION: PENDING (UPDATED | NO_CHANGE)
- FEATURE_REGISTRY_REASON_NO_CHANGE: <fill if FEATURE_REGISTRY_ACTION=NO_CHANGE>
- UI_GUIDANCE_ACTION: PENDING (UPDATED | NO_CHANGE | NOT_APPLICABLE)
- UI_GUIDANCE_REASON: <fill>
- INTERACTION_MATRIX_ACTION: PENDING (UPDATED | NO_CHANGE)
- INTERACTION_MATRIX_REASON_NO_CHANGE: <fill if INTERACTION_MATRIX_ACTION=NO_CHANGE>
- APPENDIX_MAINTENANCE_NOTES:
  - <fill>
- APPENDIX_MAINTENANCE_VERDICT: PENDING (OK | NEEDS_SPEC_UPDATE | NEEDS_STUBS)

### MECHANICAL_ENGINE_ALIGNMENT (spec-grade 22-engine set; treat each as a stand-alone feature surface)
- Rule: inspect the spec-grade mechanical engine set in Master Spec 11.8 / 6.3 as first-class force multipliers. Do not treat these engines as a vague implementation bag. If UNKNOWN, create stubs instead of guessing.
- Required rubric lines (one per engine; do not delete lines, fill values):
  - ENGINE: Spatial | ENGINE_ID: engine.spatial | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - ENGINE: Machinist | ENGINE_ID: engine.machinist | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - ENGINE: Physics | ENGINE_ID: engine.physics | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - ENGINE: Simulation | ENGINE_ID: engine.simulation | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - ENGINE: Hardware | ENGINE_ID: engine.hardware | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - ENGINE: Director | ENGINE_ID: engine.director | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - ENGINE: Composer | ENGINE_ID: engine.composer | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - ENGINE: Artist | ENGINE_ID: engine.artist | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - ENGINE: Publisher | ENGINE_ID: engine.publisher | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - ENGINE: Sous Chef | ENGINE_ID: engine.sous_chef | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - ENGINE: Food Safety | ENGINE_ID: engine.food_safety | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - ENGINE: Logistics | ENGINE_ID: engine.logistics | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - ENGINE: Archivist | ENGINE_ID: engine.archivist | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - ENGINE: Librarian | ENGINE_ID: engine.librarian | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - ENGINE: Analyst | ENGINE_ID: engine.analyst | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - ENGINE: Wrangler | ENGINE_ID: engine.wrangler | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - ENGINE: DBA | ENGINE_ID: engine.dba | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - ENGINE: Sovereign | ENGINE_ID: engine.sovereign | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - ENGINE: Guide | ENGINE_ID: engine.guide | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - ENGINE: Context | ENGINE_ID: engine.context | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - ENGINE: Version | ENGINE_ID: engine.version | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - ENGINE: Sandbox | ENGINE_ID: engine.sandbox | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
- MECHANICAL_ENGINE_ALIGNMENT_VERDICT: PENDING (OK | NEEDS_STUBS | NEEDS_SPEC_UPDATE)

### PILLAR_ALIGNMENT (Handshake pillars cross-check)
- Rule: Refinement MUST explicitly consider pillar alignment and interconnections (force multipliers). Pillar feature definition and technical implementation MUST be derived from the current Master Spec. If unknown or underspecified, write UNKNOWN and create stubs or spec updates instead of guessing.
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
  - PILLAR: Front End Memory System | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - PILLAR: Execution / Job Runtime | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - PILLAR: Prompt-to-Spec | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - PILLAR: SQL to PostgreSQL shift readiness | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - PILLAR: LLM-friendly data | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - PILLAR: Stage | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - PILLAR: Studio | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - PILLAR: Atelier/Lens | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - PILLAR: Skill distillation / LoRA | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - PILLAR: ACE | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
  - PILLAR: RAG | STATUS: <TOUCHED|NOT_TOUCHED|UNKNOWN> | NOTES: <fill> | STUB_WP_IDS: <comma-separated WP-... | NONE>
- PILLAR_ALIGNMENT_VERDICT: PENDING (OK | NEEDS_SPEC_UPDATE | NEEDS_STUBS)

### PILLAR_DECOMPOSITION (deeper pillar -> subfeature/capability slice mapping)
- Rule: for `REFINEMENT_FORMAT_VERSION >= 2026-03-08`, decompose touched or adjacent pillars into concrete capability slices so Appendix 12 can grow beyond coarse pillar rows. This is where Calendar/Loom/Locus/Stage/Studio/Atelier-Lens/Command Center/Flight Recorder/RAG mixes become explicit. Derive pillar slices and subfeatures from the current Master Spec; do not invent pillar semantics from memory. Silent omission is forbidden; every row must resolve through `IN_THIS_WP`, `NEW_STUB`, or `SPEC_UPDATE_NOW`.
- Required row format:
  - PILLAR: <fill> | CAPABILITY_SLICE: <fill> | SUBFEATURES: <fill> | PRIMITIVES_FEATURES: <comma-separated PRIM-/FEAT-/TOOL-/TECH- ids | NONE> | MECHANICAL: <comma-separated engine IDs | NONE> | ROI: <HIGH|MEDIUM|LOW> | RESOLUTION: <IN_THIS_WP|NEW_STUB|SPEC_UPDATE_NOW> | STUB: <WP-... | NONE> | NOTES: <fill>
- PILLAR_DECOMPOSITION_VERDICT: PENDING (OK | NEEDS_STUBS | NEEDS_SPEC_UPDATE)

### EXECUTION_RUNTIME_ALIGNMENT (job/workflow/tool/runtime visibility mapping)
- Rule: every new or expanded capability must map to a Handshake runtime execution surface so local models, cloud models, and operators can invoke and observe it. This section is mandatory even when `ENRICHMENT_NEEDED=NO`.
- Required row format:
  - Capability: <fill> | JobModel: <AI_JOB|WORKFLOW|MECHANICAL_TOOL|UI_ACTION|NONE> | Workflow: <fill> | ToolSurface: <UNIFIED_TOOL_SURFACE|MCP|COMMAND_CENTER|UI_ONLY|NONE> | ModelExposure: <LOCAL|CLOUD|BOTH|OPERATOR_ONLY> | CommandCenter: <VISIBLE|PLANNED|NONE> | FlightRecorder: <event ids | NONE> | Locus: <VISIBLE|PLANNED|NONE> | StoragePosture: <SQLITE_NOW_POSTGRES_READY|POSTGRES_ONLY|N/A> | Resolution: <IN_THIS_WP|NEW_STUB|SPEC_UPDATE_NOW> | Stub: <WP-... | NONE> | Notes: <fill>
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: PENDING (OK | NEEDS_STUBS | NEEDS_SPEC_UPDATE)

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

### MATRIX_RESEARCH_RUBRIC (external combo research; separate from local matrix scan)
- Rule: inspect vendor docs/papers, university/lab work, official design systems, and high-signal GitHub repos when relevant. For internal/product-governance mirror work, it is valid to mark this section `NOT_APPLICABLE` when no directly topical external combo research is needed. Link dumping is forbidden; every useful row must resolve explicitly.
- MATRIX_RESEARCH_REQUIRED: PENDING (YES | NO)
- MATRIX_RESEARCH_REASON_NO: <fill if MATRIX_RESEARCH_REQUIRED=NO>
- SOURCE_SCAN:
  - Source: <title from SOURCE_LOG> | Kind: <BIG_TECH|UNIVERSITY|PAPER|GITHUB|OSS_DOC> | Angle: <fill> | Pattern: <fill> | Decision: <ADOPT|ADAPT|REJECT> | EngineeringTrick: <fill> | ROI: <HIGH|MEDIUM|LOW> | Resolution: <IN_THIS_WP|NEW_STUB|SPEC_UPDATE_NOW|REJECT_LOW_ROI|REJECT_DUPLICATE> | Stub: <WP-... | NONE> | Notes: <fill>
- MATRIX_GROWTH_CANDIDATES:
  - Combo: <fill> | Sources: <comma-separated titles from SOURCE_LOG> | WhatToSteal: <fill> | HandshakeCarryOver: <fill> | RuntimeConsequences: <fill> | ROI: <HIGH|MEDIUM|LOW> | Resolution: <IN_THIS_WP|NEW_STUB|SPEC_UPDATE_NOW|REJECT_LOW_ROI|REJECT_DUPLICATE> | Stub: <WP-... | NONE> | Notes: <fill>
- ENGINEERING_TRICKS_CARRIED_OVER:
  - <fill>
- MATRIX_RESEARCH_VERDICT: PENDING (PASS | NOT_APPLICABLE | NEEDS_STUBS | NEEDS_SPEC_UPDATE)

### FORCE_MULTIPLIER_EXPANSION (high-ROI combinations must resolve explicitly)
- Rule: every high-ROI combination found across pillars, mechanical engines, primitives, tools, and features must end in exactly one resolution path: `IN_THIS_WP`, `NEW_STUB`, or `SPEC_UPDATE_NOW`. Silent drop is forbidden.
- COMBO_PRESSURE_MODE: AUTO
- HIGH_ROI_EXPANSION_CANDIDATES:
  - Combo: <fill> | Pillars: <comma-separated pillar names | NONE> | Mechanical: <comma-separated engine IDs | NONE> | Primitives/Features: <comma-separated PRIM-/FEAT-/TOOL-/TECH- ids | NONE> | Resolution: <IN_THIS_WP|NEW_STUB|SPEC_UPDATE_NOW> | Stub: <WP-... | NONE> | Notes: <fill>
- FORCE_MULTIPLIER_VERDICT: PENDING (OK | NEEDS_STUBS | NEEDS_SPEC_UPDATE)
- FORCE_MULTIPLIER_REASON: <fill>

### EXISTING_CAPABILITY_ALIGNMENT (dedupe against stubs, packets, UI intent, and product code)
- Rule: before creating a new stub or activating a new packet, scan existing stubs, active packets, completed packets, primitive/index coverage, interaction-matrix coverage, same-intent UI surfaces, and product code. If an equivalent capability already exists and code/UI evidence confirms it, reuse the existing artifact instead of creating a duplicate. If only partial coverage exists, expand this WP. If the gap is real, create a stub and/or spec update.
- SCAN_SCOPE: <fill>
- MATCHED_STUBS:
  - Artifact: <WP-...> | BoardStatus: STUB | Intent: <SAME|PARTIAL|DISTINCT> | PrimitiveIndex: <COVERED|MISSING|N/A> | Matrix: <COVERED|MISSING|N/A> | UI: <SAME|PARTIAL|NONE|N/A> | CodeReality: N/A | Resolution: <REUSE_EXISTING|EXPAND_IN_THIS_WP|NEW_STUB|SPEC_UPDATE_NOW|KEEP_SEPARATE> | Stub: <WP-... | NONE> | Notes: <fill>
- MATCHED_ACTIVE_PACKETS:
  - Artifact: <WP-...> | BoardStatus: <READY_FOR_DEV|IN_PROGRESS|BLOCKED> | Intent: <SAME|PARTIAL|DISTINCT> | PrimitiveIndex: <COVERED|MISSING|N/A> | Matrix: <COVERED|MISSING|N/A> | UI: <SAME|PARTIAL|NONE|N/A> | CodeReality: <PARTIAL|NOT_PRESENT|N/A> | Resolution: <REUSE_EXISTING|EXPAND_IN_THIS_WP|NEW_STUB|SPEC_UPDATE_NOW|KEEP_SEPARATE> | Stub: <WP-... | NONE> | Notes: <fill>
- MATCHED_COMPLETED_PACKETS:
  - Artifact: <WP-...> | BoardStatus: <VALIDATED|OUTDATED_ONLY|FAIL|SUPERSEDED> | Intent: <SAME|PARTIAL|DISTINCT> | PrimitiveIndex: <COVERED|MISSING|N/A> | Matrix: <COVERED|MISSING|N/A> | UI: <SAME|PARTIAL|NONE|N/A> | CodeReality: <IMPLEMENTED|PARTIAL|NOT_PRESENT> | Resolution: <REUSE_EXISTING|EXPAND_IN_THIS_WP|NEW_STUB|SPEC_UPDATE_NOW|KEEP_SEPARATE> | Stub: <WP-... | NONE> | Notes: <fill>
- CODE_REALITY_EVIDENCE:
  - Path: <repo path> | Artifact: <WP-...|NONE> | Covers: <primitive|combo|ui-intent|execution> | Verdict: <IMPLEMENTED|PARTIAL|NOT_PRESENT> | Notes: <fill>
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: PENDING (OK | REUSE_EXISTING | NEEDS_SCOPE_EXPANSION | NEEDS_STUBS | NEEDS_SPEC_UPDATE)
- EXISTING_CAPABILITY_ALIGNMENT_REASON: <fill>

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

### GUI_IMPLEMENTATION_ADVICE_RUBRIC (research-backed GUI implementation advice)
- Rule: separate hidden interaction requirements and implementation tricks from the concrete UI surface checklist. Inspect reference products/repos/design systems/papers when possible, capture hidden semantics, state models, accessibility/keyboard behavior, tooltip-vs-inline strategy, and spell out what Handshake should copy or adapt.
- GUI_ADVICE_REQUIRED: PENDING (YES | NO)
- GUI_ADVICE_REASON_NO: <fill if GUI_ADVICE_REQUIRED=NO>
- GUI_REFERENCE_SCAN:
  - Surface: <fill> | Source: <title from SOURCE_LOG or NONE> | Kind: <BIG_TECH|UNIVERSITY|PAPER|GITHUB|OSS_DOC|NONE> | Pattern: <fill> | HiddenRequirement: <fill> | InteractionContract: <fill> | Accessibility: <fill> | TooltipStrategy: <HOVER_INLINE|INLINE_PERSISTENT|MIXED|NONE> | EngineeringTrick: <fill> | Resolution: <IN_THIS_WP|NEW_STUB|SPEC_UPDATE_NOW> | Stub: <WP-... | NONE> | Notes: <fill>
- HANDSHAKE_GUI_ADVICE:
  - Surface: <fill> | Control: <fill> | Type: <fill> | Why: <fill> | Microcopy: <fill> | Tooltip: <fill>
- HIDDEN_GUI_REQUIREMENTS:
  - <fill>
- GUI_ENGINEERING_TRICKS_TO_CARRY:
  - <fill>
- GUI_IMPLEMENTATION_ADVICE_VERDICT: PENDING (PASS | NOT_APPLICABLE | NEEDS_STUBS | NEEDS_SPEC_UPDATE)

### ROADMAP_PHASE_SPLIT (only if scope must be phased)
- PHASE_SPLIT_NEEDED: PENDING (YES | NO)
- If YES: update the Roadmap (Spec 7.6) using the fixed per-phase fields below (do not invent new per-phase block types).
- Patch canonical roadmap sections in place. Do not create addendum-style normative text; use `[ADD v<version>]` markers for new lines/blocks.
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

### PACKET_HYDRATION (task packet auto-fill; mandatory for HYDRATED_RESEARCH_V1)
- PACKET_HYDRATION_PROFILE: HYDRATED_RESEARCH_V1
- REQUESTOR: Operator
- AGENT_ID: Orchestrator
- RISK_TIER: <LOW|MEDIUM|HIGH>
- SPEC_ADD_MARKER_TARGET: [ADD v<target>]
- BUILD_ORDER_DOMAIN: <BACKEND|FRONTEND|GOV|CROSS_BOUNDARY>
- BUILD_ORDER_TECH_BLOCKER: <YES|NO>
- BUILD_ORDER_VALUE_TIER: <LOW|MEDIUM|HIGH>
- BUILD_ORDER_DEPENDS_ON: <comma-separated Base WP IDs | NONE>
- BUILD_ORDER_BLOCKS: <comma-separated Base WP IDs | NONE>
- SPEC_ANCHOR_PRIMARY: <fill; packet-level anchor summary or exact anchor string>
- WHAT: <fill; 1-2 sentence scope summary>
- WHY: <fill; 1-2 sentence rationale>
- IN_SCOPE_PATHS:
  - <fill>
- OUT_OF_SCOPE:
  - <fill>
- TEST_PLAN:
  ```bash
  <fill exact commands>
  ```
- DONE_MEANS:
  - <fill>
- PRIMITIVES_EXPOSED:
  - PRIM-<fill> (or NONE)
- PRIMITIVES_CREATED:
  - PRIM-<fill> (or NONE)
- FILES_TO_OPEN:
  - <fill>
- SEARCH_TERMS:
  - <fill>
- RUN_COMMANDS:
  ```bash
  <fill exact commands>
  ```
- RISK_MAP:
  - "<risk>" -> "<impact>"
- BUILD_ORDER_SYNC_REQUIRED: PENDING (YES | NO)
- BUILD_ORDER_SYNC_NOTES:
  - <fill; if refinement changes stubs, dependencies, execution lane sequencing, or SPEC_CURRENT, sync `.GOV/roles_shared/records/BUILD_ORDER.md` before approval>

### CLAUSE_PROOF_PLAN (diff-scoped spec proof seed for coder + validator; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate the exact clauses this WP claims to satisfy, why they are in scope, where the implementation should land, what tests should prove them, and the failure mode if they are missed.
- CLAUSE_ROWS:
  - CLAUSE: <spec clause / anchor summary> | WHY_IN_SCOPE: <fill> | EXPECTED_CODE_SURFACES: <paths/symbols> | EXPECTED_TESTS: <tests/commands> | RISK_IF_MISSED: <fill>

### CONTRACT_SURFACES (serialization/producer/consumer checklist; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate every contract surface likely to drift silently across producer/consumer/validator/test boundaries.
- CONTRACT_ROWS:
  - CONTRACT: <artifact or payload> | PRODUCER: <fill> | CONSUMER: <fill> | SERIALIZER_TRANSPORT: <fill> | VALIDATOR_READER: <fill> | TRIPWIRE_TESTS: <fill> | DRIFT_RISK: <fill>

### SEMANTIC_PROOF_PLAN (diff-scoped semantic proof assets; required for REFINEMENT_FORMAT_VERSION >= 2026-03-16)
- Rule: record the concrete semantic proof assets this WP expects to rely on so later phases do not confuse green gates with semantic closure.
- Rule: each in-scope clause should be backed by one or more executable tripwires, canonical contract examples, or explicit governed debt if proof must remain partial.
- SEMANTIC_TRIPWIRE_TESTS:
  - <exact command/test/assertion target or NONE>
- CANONICAL_CONTRACT_EXAMPLES:
  - <fixture/example/golden payload/shape assertion target or NONE>

### CODER_HANDOFF_BRIEF (execution brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- IMPLEMENTATION_ORDER:
  - <fill>
- HOT_FILES:
  - <repo path>
- TRIPWIRE_TESTS:
  - <fill>
- CARRY_FORWARD_WARNINGS:
  - <fill>

### VALIDATOR_HANDOFF_BRIEF (inspection brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CLAUSES_TO_INSPECT:
  - <fill>
- FILES_TO_READ:
  - <repo path>
- COMMANDS_TO_RUN:
  - <exact command>
- POST_MERGE_SPOTCHECKS:
  - <fill or NONE>

### NOT_PROVEN_AT_REFINEMENT_TIME (explicit uncertainty ledger; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: list what refinement cannot honestly prove yet so later phases cannot silently overclaim completeness.
- NOT_PROVEN_ITEMS:
  - <fill or NONE>

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
- Rule: `ENRICHMENT_NEEDED=YES` is required both for Main Body gaps and for appendix-driven spec version bumps. Appendix-only updates still count as a spec update boundary.
- ENRICHMENT_NEEDED: PENDING
- REASON_NO_ENRICHMENT: <fill if ENRICHMENT_NEEDED=NO>

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES; includes appendix-only spec updates)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)
- Rule: for REFINEMENT_FORMAT_VERSION >= 2026-03-15, these anchor windows are also copied into the task packet `## SPEC_CONTEXT_WINDOWS` section for coder/validator downstream use.

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
