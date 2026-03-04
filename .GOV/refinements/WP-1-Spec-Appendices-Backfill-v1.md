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
- WP_ID: WP-1-Spec-Appendices-Backfill-v1
- CREATED_AT: 2026-03-04T18:46:19.130Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.141.md
- SPEC_TARGET_SHA1: f3b0715a544ebae689bee2196c0a4041cf4f2798
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja040320262011
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Spec-Appendices-Backfill-v1

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- EOF Appendix blocks exist but are effectively empty (bootstrap-only) for Phase 1: FEATURE_REGISTRY contains only FEAT-SPEC-APPENDICES; PRIMITIVE_TOOL_TECH_MATRIX has no primitives/tools/technologies/links; UI_GUIDANCE covers only the appendix system; INTERACTION_MATRIX has no edges.
- The spec currently lacks a comprehensive in-spec inventory of "current primitives" (traits/structs/enums + cross-cutting system primitives) and their relationships to features.
- Without backfill, external LLMs and humans do not have a stable, machine-readable index/matrix to avoid feature and UI guidance gaps.

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: 30m (initial pass; deepen only if we discover incompatible patterns)
- REFERENCES:
  - OpenAPI / JSON Schema patterns for machine-readable registries (schema versioning, stable IDs, regeneratable derived views).
  - Backstage Software Catalog (entity registry + relationships + ownership metadata).
  - Kubernetes API conventions (stable identifiers, compatibility, and separation of spec vs generated views).
  - Terraform provider schema approach (explicit resource inventory + schema-driven tooling).
  - Stripe/Cloudflare style public API docs patterns (explicit "surface" separation and stable naming).
- PATTERNS_EXTRACTED:
  - Stable IDs: treat identifiers as durable; do not rename; deprecate instead.
  - Deterministic ordering: sort arrays/objects to reduce merge conflicts and drift.
  - Schema versioning: include schema/version fields to allow evolution without breaking parsers.
  - Separation of concerns: index/matrix should be derived views of Main Body truths, not new normative requirements.
  - Relationship edges: express interactions explicitly as typed edges rather than prose-only references.
- DECISIONS (ADOPT/ADAPT/REJECT):
  - ADOPT: stable IDs + schema/version fields (already present; extend consistently).
  - ADOPT: deterministic ordering + minimal notes fields to reduce churn.
  - ADAPT: relationship edges (start small: high-signal edges only; expand incrementally).
  - REJECT (for now): external authoritative registries outside the spec; keep spec self-contained per CX-SPEC-APPX-003.
- LICENSE/IP_NOTES: No code-level reuse planned; reference-only patterns. If we later import schema snippets or code generators, validate licenses first.
- SPEC_IMPACT: NO
- SPEC_IMPACT_REASON: This WP backfills data into existing EOF appendix blocks without changing Main Body requirements or schemas.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- NONE (spec-only inventory/matrix backfill; no runtime behavior change).

### RED_TEAM_ADVISORY (security failure modes)
- Risk: Appendix drift or inaccurate entries could mislead implementers/LLMs into shipping UI without correct guardrails. Mitigation: treat appendices as derived from Main Body; keep entries anchored to spec sections; prefer minimal, verifiable claims; create remediation stubs for any uncertainty rather than guessing.
- Risk: Merge conflict surface grows as JSON blocks grow. Mitigation: deterministic sorting and small, incremental updates per change; avoid large-format rewrites.

### PRIMITIVES (traits/structs/enums)
- Spec-level primitives only (no runtime code changes in this WP):
  - feature_id (stable string ID; never reuse)
  - primitive_id / tool_id / technology_id (stable string IDs; never reuse)
  - interaction edge (typed feature<->feature, feature<->primitive, feature<->tool/tech links)

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Master Spec v02.140 Section 12 defines the EOF appendix system (feature registry, primitive/tool/tech matrix, UI guidance, interaction matrix) and explicitly acknowledges legacy backfill as a stub WP, making this backfill work both intended and in-scope.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The Master Spec already defines the EOF appendix blocks and schemas; this WP populates/backfills the data inside those existing blocks without adding new normative Main Body requirements.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.141.md 12.2 Maintenance rules (HARD) [CX-SPEC-APPX-003]
- CONTEXT_START_LINE: 70261
- CONTEXT_END_LINE: 70271
- CONTEXT_TOKEN: Legacy features MAY be missing UI guidance until backfilled
- EXCERPT_ASCII_ESCAPED:
  ```text
  # 12. End-of-File Appendices (Feature Index + Matrix + UI Guidance) [CX-SPEC-APPX-001]
  
  ## 12.2 Maintenance rules (HARD) [CX-SPEC-APPX-003]
  
  Hard invariants:
  1. The Master Spec remains the source of truth. These appendices are inside this file to preserve that.
  2. The appendix blocks MUST be the last major section in the file (end-of-file). Do not move them earlier.
  3. Each appendix block MUST be a fenced block bracketed by BEGIN/END markers with a unique id and schema version.
  4. UI guidance is REQUIRED for new/changed features only:
     - If a feature is introduced or materially changed, its UI guidance entry MUST be added/updated in the UI guidance appendix.
     - Legacy features MAY be missing UI guidance until backfilled; track backfill as a stub WP (do not block unrelated work).
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.141.md 12.3 HS-APPX-FEATURE-REGISTRY [CX-SPEC-APPX-010]
- CONTEXT_START_LINE: 70272
- CONTEXT_END_LINE: 70286
- CONTEXT_TOKEN: HS-APPX-FEATURE-REGISTRY
- EXCERPT_ASCII_ESCAPED:
  ```text
  ## 12.3 Appendix Block: FEATURE_REGISTRY (Machine-readable) [CX-SPEC-APPX-010]
  
  <!-- HS_APPENDIX:BEGIN id=HS-APPX-FEATURE-REGISTRY schema=hs_feature_registry@1 -->
  ```json
  {
    "schema": "hs_feature_registry@1",
    "spec_version": "v02.141",
    "last_updated": "2026-03-04",
    "features": [
      {
        "feature_id": "FEAT-ACE-RUNTIME",
        "title": "ACE Runtime (Agentic Context Engineering)",
        "spec_anchor": "\\u00A72.6.6.7"
      }
    ]
  }
  ```
  <!-- HS_APPENDIX:END id=HS-APPX-FEATURE-REGISTRY -->
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.141.md 12.4 HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX [CX-SPEC-APPX-011]
- CONTEXT_START_LINE: 70529
- CONTEXT_END_LINE: 70543
- CONTEXT_TOKEN: HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX
- EXCERPT_ASCII_ESCAPED:
  ```text
  ## 12.4 Appendix Block: PRIMITIVE_TOOL_TECH_MATRIX (Machine-readable) [CX-SPEC-APPX-011]
  
  <!-- HS_APPENDIX:BEGIN id=HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX schema=hs_primitive_tool_tech_matrix@1 -->
  ```json
  {
       "schema":  "hs_primitive_tool_tech_matrix@1",
       "spec_version":  "v02.141",
       "last_updated":  "2026-03-04",
       "primitives":  [
                          {
                              "primitive_id":  "PRIM-AccessMode",
                              "title":  "AccessMode",
                              "kind":  "rust_enum"
                          },
                          {
                              "primitive_id":  "PRIM-AceRuntimeValidator",
                              "title":  "AceRuntimeValidator",
                              "kind":  "rust_trait"
                          }
                      ]
  }
  ```
  <!-- HS_APPENDIX:END id=HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX -->
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.141.md 12.5 HS-APPX-UI-GUIDANCE [CX-SPEC-APPX-012]
- CONTEXT_START_LINE: 73273
- CONTEXT_END_LINE: 73292
- CONTEXT_TOKEN: HS-APPX-UI-GUIDANCE
- EXCERPT_ASCII_ESCAPED:
  ```text
  ## 12.5 Appendix Block: UI_GUIDANCE (Per Feature) [CX-SPEC-APPX-012]
  
  <!-- HS_APPENDIX:BEGIN id=HS-APPX-UI-GUIDANCE schema=hs_ui_guidance@1 -->
  ```json
  {
    "schema": "hs_ui_guidance@1",
    "spec_version": "v02.141",
    "last_updated": "2026-03-04",
    "ui_guidance": [
      {
        "feature_id": "FEAT-CALENDAR",
        "user_goal": "View and author local calendar events and correlate time ranges with Activity/Session spans."
      }
    ]
  }
  ```
  <!-- HS_APPENDIX:END id=HS-APPX-UI-GUIDANCE -->
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.141.md 12.6 HS-APPX-INTERACTION-MATRIX [CX-SPEC-APPX-013]
- CONTEXT_START_LINE: 73433
- CONTEXT_END_LINE: 73449
- CONTEXT_TOKEN: HS-APPX-INTERACTION-MATRIX
- EXCERPT_ASCII_ESCAPED:
  ```text
  ## 12.6 Appendix Block: INTERACTION_MATRIX (Feature/Primitive edges) [CX-SPEC-APPX-013]
  
  <!-- HS_APPENDIX:BEGIN id=HS-APPX-INTERACTION-MATRIX schema=hs_interaction_matrix@1 -->
  ```json
  {
    "schema": "hs_interaction_matrix@1",
    "spec_version": "v02.141",
    "last_updated": "2026-03-04",
    "edges": [
      {
        "edge_id": "IMX-001",
        "from_kind": "feature",
        "from_id": "FEAT-UNIFIED-TOOL-SURFACE",
        "to_kind": "feature",
        "to_id": "FEAT-CAPABILITIES-CONSENT",
        "kind": "enforce_capabilities",
        "scope": "normative",
        "tokens": ["HTC-1.0", "session-scoped capability intersection"],
        "spec_refs": ["\\u00A76.0.2", "#111-capabilities-consent-model"]
      }
    ]
  }
  ```
  <!-- HS_APPENDIX:END id=HS-APPX-INTERACTION-MATRIX -->
  ```
