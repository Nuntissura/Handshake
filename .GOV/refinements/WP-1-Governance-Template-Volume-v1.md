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
- WP_ID: WP-1-Governance-Template-Volume-v1
- CREATED_AT: 2026-01-16T02:21:49Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.112.md
- SPEC_TARGET_SHA1: 33b50fe7d70381c3eb2a53871f673e1d442633e1
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja160120260327
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Governance-Template-Volume-v1

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- NONE. The Master Spec already defines (a) project-agnostic Governance Pack instantiation, (b) the full Template Volume with a placeholder glossary, and (c) the unified export/materialize contract ("path chosen by user") needed for directory export.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- Governance Pack export MUST emit an ExportRecord (per 2.3.10) at the end of the export run ("Write an ExportRecord to Flight Recorder / workspace logs"), including determinism_level and the user-chosen LocalFile materialized path(s).

### RED_TEAM_ADVISORY (security failure modes)
- Path traversal: refuse absolute paths and any ".." segments in template paths; enforce export root confinement.
- Overwrite/destruction: default-deny overwriting non-empty directories; require explicit operator confirmation for overwrite mode.
- Leakage: never include secrets in any export manifest/log fields; avoid emitting absolute filesystem paths unless required by ExportRecord semantics; prefer relative paths where possible.
- Drift: prevent exporting from the current repo working tree as a "source of truth"; use the canonical Template Volume defined in the current Master Spec to avoid silent divergence.
- Determinism: ensure placeholder resolution and file write order are stable; avoid OS-dependent behaviors (path separators, line endings) in exported bytes.

### PRIMITIVES (traits/structs/enums)
- GovernancePackTemplate (path + body; extracted from Template Volume).
- GovernancePackTemplateIndex (ordered list of templates; stable ordering).
- PlaceholderResolver (fills the 7.5.4.9.1 glossary placeholders from project invariants).
- GovernancePackExporter (writes resolved templates to a user-selected export directory under determinism + safety constraints).

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS (Template Index defines required file set; export contract requires ExportRecord; placeholders are enumerated)
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: The spec provides a full inlined Template Volume (7.5.4.9.3) plus a placeholder glossary (7.5.4.9.1) and explicitly states these are canonical templates that MUST be rendered with project-specific values; it also defines a unified export/materialize contract (2.3.10) including "path chosen by user" and ExportRecord emission.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The Master Spec already includes the complete template bodies and the required placeholder set, and it already defines export/materialize + ExportRecord requirements. Implementation decisions (parser strategy, UI prompt surface, caching) are allowed as long as the exported file set matches the Template Volume and the export contract is obeyed.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.112.md 7.5.4.8 (Governance Pack: Project-Specific Instantiation) (HARD)
- CONTEXT_START_LINE: 20517
- CONTEXT_END_LINE: 20574
- CONTEXT_TOKEN: pub struct ProjectIdentity {
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 7.5.4.8 Governance Pack: Project-Specific Instantiation (HARD)

  **Purpose**
  Handshake MUST implement the project-agnostic Governance Kernel (7.5.4; `.GOV/GOV_KERNEL/*`) as a project-parameterized Governance Pack so the same strict workflow can be generated and enforced for arbitrary projects (not Handshake-specific).

  **Project identity (normative)**

  ```rust
  pub struct ProjectIdentity {
      pub project_code: String,            // short stable prefix, e.g. "COOK"
      pub project_display_name: String,    // human name
      pub naming_policy: NamingPolicy,
      pub language_layout_profile_id: String,
      pub role_mailbox_export_dir: String,      // MUST default to ".GOV/ROLE_MAILBOX/"
      pub external_tool_paths: ExternalToolPaths,
  }
  ```
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.112.md 7.5.4.9 (Governance Pack: Template Volume) (HARD)
- CONTEXT_START_LINE: 20578
- CONTEXT_END_LINE: 20620
- CONTEXT_TOKEN: ##### 7.5.4.9.1 Placeholder Glossary (HARD)
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 7.5.4.9 Governance Pack: Template Volume (HARD)

  **Hard rule (Instantiation)**
  - The Governance Pack export MUST include these templates with ALL placeholders resolved.
  - The exported repo MUST provide a single deterministic command surface (e.g., `just pre-work`, `just post-work`, `just validate-workflow`).
  - Project-specific naming/layout/tool paths MUST live in `.GOV/roles_shared/PROJECT_INVARIANTS.md` (do not hardcode in templates).

  ##### 7.5.4.9.1 Placeholder Glossary (HARD)
  - `{{PROJECT_CODE}}`: short stable code, e.g., `COOK`.
  - `{{PROJECT_DISPLAY_NAME}}`: human name, e.g., `Cooking App`.
  - `{{ISSUE_PREFIX}}`: issue prefix for TODO tagging, e.g., `COOK` (used as `TODO({{ISSUE_PREFIX}}-1234)` / error codes).
  - `{{FRONTEND_ROOT_DIR}}`, `{{FRONTEND_SRC_DIR}}`: frontend layout roots (project-specific).
  - `{{BACKEND_ROOT_DIR}}`, `{{BACKEND_CRATE_DIR}}`, `{{BACKEND_SRC_DIR}}`, `{{BACKEND_MIGRATIONS_DIR}}`: backend layout roots (project-specific).
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.112.md 7.5.4.9.3 (Template Bodies) markers (HARD)
- CONTEXT_START_LINE: 20680
- CONTEXT_END_LINE: 20690
- CONTEXT_TOKEN: <!-- GOV_PACK_TEMPLATE_VOLUME_BEGIN -->
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 7.5.4.9.3 Template Bodies (HARD)
  <!-- GOV_PACK_TEMPLATE_VOLUME_BEGIN -->
  ...
  <!-- GOV_PACK_TEMPLATE_VOLUME_END -->
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.112.md 2.3.10.1-2.3.10.2 (Export pipeline + ExportRecord) (Normative)
- CONTEXT_START_LINE: 2424
- CONTEXT_END_LINE: 2460
- CONTEXT_TOKEN: Write an ExportRecord
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 2.3.10 Export & Artifact Production (Unified Contract)

  #### 2.3.10.1 Canonical export pipeline (normative)

  1. Select sources by EntityRef (Raw/Derived content is never edited by export).
  2. Build a Display projection (DisplayContent/layout decisions).
  3. Apply ExportGuard for the chosen ExportTarget.
  4. Run exporter (mechanical job) to produce one or more ArtifactHandles.
  5. (Optional) Materialize to a path (LocalFile) or pass the artifact to a connector.
  6. Write an ExportRecord to Flight Recorder / workspace logs.
  ```


