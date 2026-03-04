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
- WP_ID: WP-1-Spec-Appendices-Index-Matrix-UI-Guidance-v1
- CREATED_AT: 2026-03-04
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.139.md
- SPEC_TARGET_SHA1: 0a5a9069bf8e06654ddf9b647927c2cb8a30aa6f
- USER_REVIEW_STATUS: PENDING
- USER_SIGNATURE: <pending>
- USER_APPROVAL_EVIDENCE: <pending> (must equal: APPROVE REFINEMENT WP-1-Spec-Appendices-Index-Matrix-UI-Guidance-v1)

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- The Master Spec is authoritative, but it lacks an explicit, machine-readable end-of-file "feature index + interaction matrix + per-feature UI guidance" inventory. This causes:
  - cognitive load spikes as the feature surface grows,
  - missed UI affordances (features exist but no UI contract is written),
  - missed interactions (primitives/features/tools/technology can drift into incompatible assumptions),
  - non-deterministic coverage (hard to prove "UI guidance exists for new/changed features" in review/validation).

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- NONE (this WP is spec + governance workflow only; no runtime telemetry changes required).

### RED_TEAM_ADVISORY (security failure modes)
- If UI guidance is not treated as a spec-level contract, high-risk actions can be exposed without explicit UX guardrails (preview-before-apply, stale-state detection, capability/consent gating), increasing the chance of destructive or consent-violating flows.
- If the interaction matrix is not tracked, cross-feature integration can silently introduce data exfiltration paths (e.g., a feature consuming artifacts without honoring capability scope).
- If appendices are maintained outside the spec (or without deterministic markers), they can drift and become an alternate "truth" that conflicts with Main Body requirements.

### PRIMITIVES (traits/structs/enums)
- NONE (spec documentation / schema definitions only).

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [ ] PASS
- Explicitly named: [ ] PASS
- Specific: [ ] PASS
- Measurable acceptance criteria: [ ] PASS
- No ambiguity: [ ] PASS
- CLEARLY_COVERS_VERDICT: FAIL
- CLEARLY_COVERS_REASON: The current spec does not define a single, end-of-file, machine-readable feature inventory + interaction matrix + per-feature UI guidance contract. This WP adds that missing contract surface.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: YES
- REASON_NO_ENRICHMENT: N/A

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
````md
## Table of Contents (Enrichment)

Add this entry to the Table of Contents list:
- [12 End-of-File Appendices (Feature Index + Matrix + UI Guidance)](#12-end-of-file-appendices)

---

<a id="12-end-of-file-appendices"></a>
# 12. End-of-File Appendices (Feature Index + Matrix + UI Guidance) [CX-SPEC-APPX-001]

## 12.1 Why these appendices exist [CX-SPEC-APPX-002]

Handshake is an IDE and an execution harness. As the number of features/primitives/tools/technologies grows, the spec needs an explicit inventory that:
- keeps the Master Spec self-contained (no normative dependence on external derived files),
- forces per-feature UI guidance to exist (so features do not ship without an interaction contract),
- makes interactions explicit (so "everything can use everything" remains safe and coherent),
- reduces cognitive load for humans and external LLMs by providing a stable index and matrix.

These appendices live at the end of the Master Spec so that:
- the Main Body remains the primary reading flow,
- the appendices can be treated as a stable, parseable contract surface,
- derived views can be generated without changing meaning.

## 12.2 Maintenance rules (HARD) [CX-SPEC-APPX-003]

Hard invariants:
1. The Master Spec remains the source of truth. Appendices are inside this file to preserve that.
2. The appendix blocks MUST be the last major section in the file (end-of-file). Do not move them earlier.
3. Each appendix block MUST be a fenced block bracketed by BEGIN/END markers with a unique id and schema version.
4. UI guidance is REQUIRED for new/changed features only:
   - If a feature is introduced or materially changed, its UI guidance entry MUST be added/updated in the UI guidance appendix.
   - Legacy features MAY be missing UI guidance until backfilled; track backfill as a stub WP (do not block unrelated work).
5. External derived files (indexes/matrices extracted into repo folders) are allowed, but MUST be explicitly labeled DERIVED and MUST be regeneratable from this spec. They are never normative.

## 12.3 Appendix Block: FEATURE_REGISTRY (Machine-readable) [CX-SPEC-APPX-010]

<!-- HS_APPENDIX:BEGIN id=HS-APPX-FEATURE-REGISTRY schema=hs_feature_registry@1 -->
```json
{
  "schema": "hs_feature_registry@1",
  "spec_version": "v02.140",
  "last_updated": "2026-03-04",
  "features": [
    {
      "feature_id": "FEAT-SPEC-APPENDICES",
      "title": "End-of-File Spec Appendices System",
      "spec_anchor": "#12-end-of-file-appendices",
      "surfaces": ["spec"],
      "primitives": [],
      "tools_tech": ["markdown"],
      "notes": "This entry exists to bootstrap the appendix system itself."
    }
  ]
}
```
<!-- HS_APPENDIX:END id=HS-APPX-FEATURE-REGISTRY -->

## 12.4 Appendix Block: PRIMITIVE_TOOL_TECH_MATRIX (Machine-readable) [CX-SPEC-APPX-011]

<!-- HS_APPENDIX:BEGIN id=HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX schema=hs_primitive_tool_tech_matrix@1 -->
```json
{
  "schema": "hs_primitive_tool_tech_matrix@1",
  "spec_version": "v02.140",
  "last_updated": "2026-03-04",
  "primitives": [],
  "tools": [],
  "technologies": [],
  "feature_links": []
}
```
<!-- HS_APPENDIX:END id=HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX -->

## 12.5 Appendix Block: UI_GUIDANCE (Per Feature) [CX-SPEC-APPX-012]

<!-- HS_APPENDIX:BEGIN id=HS-APPX-UI-GUIDANCE schema=hs_ui_guidance@1 -->
```json
{
  "schema": "hs_ui_guidance@1",
  "spec_version": "v02.140",
  "last_updated": "2026-03-04",
  "ui_guidance": [
    {
      "feature_id": "FEAT-SPEC-APPENDICES",
      "user_goal": "Maintain a self-contained spec that scales: explicit feature inventory, explicit interaction model, explicit per-feature UI contract.",
      "entry_points": ["Master Spec Section 12"],
      "required_surfaces": ["spec"],
      "interaction_contract": {
        "states": ["present", "stale_derived_views"],
        "errors": ["appendix_missing", "schema_invalid", "derived_drift"]
      },
      "capability_gates": [],
      "telemetry": [],
      "tests": [
        "gov-check includes an appendix presence + json-parse validation (repo-level, deterministic)"
      ]
    }
  ]
}
```
<!-- HS_APPENDIX:END id=HS-APPX-UI-GUIDANCE -->

## 12.6 Appendix Block: INTERACTION_MATRIX (Feature/Primitive edges) [CX-SPEC-APPX-013]

<!-- HS_APPENDIX:BEGIN id=HS-APPX-INTERACTION-MATRIX schema=hs_interaction_matrix@1 -->
```json
{
  "schema": "hs_interaction_matrix@1",
  "spec_version": "v02.140",
  "last_updated": "2026-03-04",
  "edges": []
}
```
<!-- HS_APPENDIX:END id=HS-APPX-INTERACTION-MATRIX -->
````

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.139.md Table of Contents
- CONTEXT_START_LINE: 100
- CONTEXT_END_LINE: 190
- CONTEXT_TOKEN: ## Table of Contents
- EXCERPT_ASCII_ESCAPED:
  ```text
  ## Table of Contents
  
  - [1 Vision & Context](#1-vision-context)
    - [1.1 Executive Summary](#11-executive-summary)
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.139.md End of Addendum v0.5 (EOF insertion point)
- CONTEXT_START_LINE: 70220
- CONTEXT_END_LINE: 70239
- CONTEXT_TOKEN: *End of Addendum v0.5 (Autonomous Orchestration Integration + Role Mailbox Alignment)*
- EXCERPT_ASCII_ESCAPED:
  ```text
  *End of Addendum v0.5 (Autonomous Orchestration Integration + Role Mailbox Alignment)*
  ```
