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
- WP_ID: WP-1-Product-Governance-Snapshot-v1
- CREATED_AT: 2026-02-05T04:14:09.945Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.123.md
- SPEC_TARGET_SHA1: 3723a94e78d7d6274b85455b864af15c784d81e2
- USER_REVIEW_STATUS: PENDING
- USER_SIGNATURE: <pending>
- USER_APPROVAL_EVIDENCE: <pending> (must equal: APPROVE REFINEMENT WP-1-Product-Governance-Snapshot-v1)

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- Spec uses `.GOV/...` as canonical Governance Pack export paths (main body), but Operator policy requires a hard split between repo governance workspace and product runtime.
- Current product implementation historically read `docs/**` for governance-critical defaults/state; spec does not explicitly permit this and does not explicitly forbid repo-relative governance I/O.
- Path semantics are underspecified: does `.GOV/...` in the Master Spec mean (a) repo governance workspace, or (b) a generated governance pack output directory inside an arbitrary target project?

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- NONE (this WP is about resource packaging + path boundary enforcement; do not introduce new event IDs).
- Risk note: if any Flight Recorder schemas record export roots/paths, ensure path changes do not break schema validation.

### RED_TEAM_ADVISORY (security failure modes)
- Prevent path traversal: any configured export directory MUST be normalized and MUST NOT allow `..` escapes outside the intended workspace/data root.
- Do not trust repo-relative paths for runtime defaults; embed/bundle defaults to prevent malicious repo content influencing runtime behavior.
- Avoid writing governance exports into a developer-governance workspace directory by default (prevents data clobber + audit confusion).

### PRIMITIVES (traits/structs/enums)
- Expect new product primitives for embedded governance resources (e.g., `EmbeddedGovernancePack`, `EmbeddedDoc`, `GovernanceDefaults`).
- Expect a product-owned storage location abstraction for exports/state (configurable, not repo-relative).

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [ ] FAIL
- Explicitly named: [ ] FAIL
- Specific: [ ] FAIL
- Measurable acceptance criteria: [ ] FAIL
- No ambiguity: [ ] FAIL
- CLEARLY_COVERS_VERDICT: FAIL
- CLEARLY_COVERS_REASON: Master Spec defines Governance Pack exports under `.GOV/...` but does not clearly define the boundary between (1) repo governance workspace and (2) product runtime defaults/state. Operator requirement is a hard split: product MUST NOT rely on repo `.GOV/` or repo `docs/`.
- AMBIGUITY_FOUND: YES
- AMBIGUITY_REASON: `.GOV/...` is used in the spec as a canonical path, but Operator requirement forbids product runtime touching `.GOV/`. This must be clarified normatively (runtime vs repo workspace, and safe export root defaults).

### ENRICHMENT
- ENRICHMENT_NEEDED: YES
- REASON_NO_ENRICHMENT: <not applicable>

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
[ADD] 7.5.4.8.X Runtime vs Repo Governance Boundary (HARD)

Handshake MUST enforce a hard split between:
- the repo governance workspace used for human/LLM workflow enforcement, and
- the product runtime implementation.

Hard requirements:
- The Handshake product runtime MUST NOT read governance defaults/templates from repo-local directories such as `.GOV/` or `docs/`.
- Governance Pack templates MUST be shipped with the product build as embedded/bundled resources (versioned with the binary/library).
- Repo governance workspaces (e.g., this repository's `.GOV/`) are NOT a runtime dependency contract.
- Any on-disk governance exports (e.g., role mailbox export, validator gate ledgers) MUST use a product-owned data directory (configurable) and MUST NOT default to a repo governance workspace directory name.

Update ProjectIdentity default (normative):
- Replace:
  - `pub role_mailbox_export_dir: String,      // MUST default to ".GOV/ROLE_MAILBOX/"`
- With:
  - `pub role_mailbox_export_dir: String,      // MUST default to ".handshake/ROLE_MAILBOX/" (product-owned data dir)`

Compatibility note (non-normative):
- This repository MAY keep a legacy `docs/` directory to support older implementations/tests, but the product runtime MUST NOT depend on it.
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.123.md 7.5.4.8 (Governance Pack: Project-Specific Instantiation)
- CONTEXT_START_LINE: 28453
- CONTEXT_END_LINE: 28510
- CONTEXT_TOKEN: MUST default to ".GOV/ROLE_MAILBOX/"
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 7.5.4.8 Governance Pack: Project-Specific Instantiation (HARD)

  **Purpose**
  Handshake MUST implement the project-agnostic Governance Kernel (7.5.4; `.GOV/GOV_KERNEL/*`) as a project-parameterized **Governance Pack** so the same strict workflow can be generated and enforced for arbitrary projects (not Handshake-specific).

  **Definitions**
  - **Governance Pack**: a versioned bundle of templates + gate semantics that instantiate:
    - project codex,
    - role protocols,
    - canonical governance artifacts and templates,
    - mechanical gate tooling (.GOV/scripts/hooks/CI) and a single command surface (e.g., `just`),
    - deterministic exports (including `.GOV/ROLE_MAILBOX/` when enabled by governance mode).

  pub struct ProjectIdentity {
      pub project_code: String,
      pub project_display_name: String,
      pub naming_policy: NamingPolicy,
      pub language_layout_profile_id: String,
      pub role_mailbox_export_dir: String,      // MUST default to ".GOV/ROLE_MAILBOX/"
      pub external_tool_paths: ExternalToolPaths,
  }

  **Kernel parity rule (HARD)**
  Any project claiming Governance Kernel conformance MUST be able to reconstruct, from canonical artifacts alone:
  - role mailbox transcripts (if used) via `.GOV/ROLE_MAILBOX/`.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.123.md 7.5.4.9.2 (Template Index: validator gate state model)
- CONTEXT_START_LINE: 28542
- CONTEXT_END_LINE: 28566
- CONTEXT_TOKEN: `.GOV/validator_gates/{WP_ID}.json`
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 7.5.4.9.2 Template Index (HARD)
  | Path | Intent |
  |------|--------|
  | `.GOV/roles_shared/SIGNATURE_AUDIT.md` | Central registry of consumed USER_SIGNATURE tokens (anti-replay / audit trail). |
  | `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json` | Mechanical Orchestrator gate state model (initial empty state). |
  | `.GOV/validator_gates/{WP_ID}.json` | Mechanical Validator gate state model (per-WP; merge-safe). |
  ```
