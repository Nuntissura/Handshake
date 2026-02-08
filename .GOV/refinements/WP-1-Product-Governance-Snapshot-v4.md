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
- WP_ID: WP-1-Product-Governance-Snapshot-v4
- CREATED_AT: 2026-02-08T19:37:30.442Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.125.md
- SPEC_TARGET_SHA1: d16eb1eb5045e858112b2ce477f27aa0200621b0
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja080220262058
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Product-Governance-Snapshot-v4

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- Implementation divergence: the v3 implementation of Product Governance Snapshot is repo-governance-centric (`/.GOV/**` inputs/outputs), while the product runtime still relies on repo paths (`docs/**` and/or `/.GOV/**`) for governance-critical defaults/state in practice (boundary violation).
- Lineage divergence: the v1 stub intent required a hard split (repo governance workspace vs product runtime) with runtime governance state in product-owned storage; that intent was not carried into v3 behavior.
- This v4 WP is remediation to carry forward BOTH: (a) v3 snapshot determinism/leak-safety requirements and (b) v1 decouple + product-owned runtime governance state boundary.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- NONE. This WP changes governance boundary/state path semantics and snapshot generation/validation; it must not introduce new Flight Recorder event IDs unless already required by the Master Spec for the affected subsystem.

### RED_TEAM_ADVISORY (security failure modes)
- Repo path injection: if runtime reads `.GOV/**` or `docs/**`, a malicious repo checkout can influence runtime governance behavior. Enforce the hard boundary and embed/bundle runtime defaults.
- Leak risk: snapshot and mailbox export must not include secrets/env vars/raw message bodies; include only hashes/refs per spec.
- Path traversal: any configurable runtime governance state dir must be normalized and must not allow `..` escapes outside the intended data root.

### PRIMITIVES (traits/structs/enums)
- Rust (expected):
  - `ProductGovernanceSnapshot` schema (v0.1) generator/validator (may already exist from v3; ensure it remains deterministic and leak-safe).
  - Runtime governance state path resolver (default `.handshake/gov/`, configurable) used by governance pack / mailbox / workflow components.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [ ] PASS
- Explicitly named: [ ] PASS
- Specific: [ ] PASS
- Measurable acceptance criteria: [ ] PASS
- No ambiguity: [ ] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Master Spec v02.125 defines the repo/runtime boundary and runtime governance state location in 7.5.4.8 (HARD) and defines Product Governance Snapshot inputs/output/determinism/leak-safety in 7.5.4.10 (HARD). This WP is implementation remediation against those normative requirements; no spec enrichment required.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The boundary rules (`/.GOV/` workspace vs runtime) and runtime governance state default `.handshake/gov/` are explicitly defined in 7.5.4.8; snapshot requirements are defined in 7.5.4.10.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.125.md 7.5.4.8 Governance Pack: Project-Specific Instantiation (HARD) (repo/runtime boundary)
- CONTEXT_START_LINE: 28455
- CONTEXT_END_LINE: 28518
- CONTEXT_TOKEN: .handshake/gov/
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
    - mechanical gate tooling (scripts/hooks/CI) and a single command surface (e.g., `just`),
    - deterministic exports (including `.GOV/ROLE_MAILBOX/` when enabled by governance mode).

  **Project identity (normative)**

  ```rust
  pub struct ExternalToolPaths {
      pub cargo_target_dir: Option<String>, // project-specific; may be external
      pub node_package_manager: Option<String>,
      pub additional_paths: std::collections::HashMap<String, String>,
  }

  pub struct NamingPolicy {
      // Recommended defaults: underscore-separated, no spaces (shell/OS safe; deterministic parsing).
      pub master_spec_pattern: String, // e.g. "<PROJECT>_Master_Spec_vNN.NNN.md"
      pub codex_pattern: String,       // e.g. "<PROJECT>_Codex_vX.Y.md"
  }

  pub struct ProjectIdentity {
      pub project_code: String,            // short stable prefix, e.g. "COOK"
      pub project_display_name: String,    // human name
      pub naming_policy: NamingPolicy,
      pub language_layout_profile_id: String,   // always present; project-specific (no Handshake-hardcoded paths)
      pub role_mailbox_export_dir: String,      // MUST default to ".GOV/ROLE_MAILBOX/"
      pub external_tool_paths: ExternalToolPaths,
  }
  ```

  **Invariants (HARD)**
  - Language/layout guardrails MUST always exist and MUST be project-specific (no Handshake-hardcoded paths).
  - External tool paths MUST be explicit, prompted/configured per project, and persisted (workspace settings and repo-exported identity).
  - The Governance Pack MUST NOT hardcode `Handshake_*` filenames when instantiating non-Handshake projects.
  - For GOV_STANDARD and GOV_STRICT, the Trinity roles MUST be enforced (11.1.5.1).

  **Conformance and alternate implementations (HARD)**
  - Node/just/bash reference implementations are allowed and preferred for strict determinism.
  - Alternate implementations (different language/tooling) are allowed ONLY if:
    - they enforce the same semantics,
    - they are deterministic,
    - and they ship a conformance proof (tests/harness) plus an explicit "intent" note describing equivalence and any deviations.

  **Kernel parity rule (HARD)**
  Any project claiming Governance Kernel conformance MUST be able to reconstruct, from canonical artifacts alone:
  - current authoritative spec,
  - authorized work and scope,
  - evidence and remaining gates,
  - active/in-progress/done/stub state,
  - role mailbox transcripts (if used) via `.GOV/ROLE_MAILBOX/`.

  **Repo/runtime boundary (HARD)**  
  - `/.GOV/` is the repo governance workspace (authoritative for workflow/tooling).
  - `docs/` MAY exist as a temporary compatibility bundle only (non-authoritative governance state).
  - Handshake product runtime MUST NOT read from or write to `/.GOV/` (hard boundary; enforce via CI/gates).
  - Runtime governance state MUST live in product-owned storage. Handshake default: `.handshake/gov/` (configurable). This directory contains runtime governance state only.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.125.md 7.5.4.10 Product Governance Snapshot (HARD)
- CONTEXT_START_LINE: 42852
- CONTEXT_END_LINE: 42902
- CONTEXT_TOKEN: .GOV/roles_shared/PRODUCT_GOVERNANCE_SNAPSHOT.json
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 7.5.4.10 Product Governance Snapshot (HARD)

  **Purpose**  
  Provide a deterministic, leak-safe snapshot of the current governance state for a product/repo so a fresh agent (or auditor) can reconstruct "what is true" without relying on chat history.

  **Definition**  
  A "Product Governance Snapshot" is a machine-readable JSON export derived ONLY from canonical governance artifacts (no repo scan; no extras):
  - `.GOV/roles_shared/SPEC_CURRENT.md`
  - resolved spec file referenced inside it (e.g., `Handshake_Master_Spec_v02.125.md`)
  - `.GOV/roles_shared/TASK_BOARD.md`
  - `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`
  - `.GOV/roles_shared/SIGNATURE_AUDIT.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json`
  - `.GOV/validator_gates/*.json` (if present)

  **Output location (HARD)**  
  - Default path: `.GOV/roles_shared/PRODUCT_GOVERNANCE_SNAPSHOT.json`
  - The export MUST be deterministic for a given set of input files.
  - The export MUST NOT include wall-clock timestamps.
  - The export MAY include the current git HEAD sha (if available) as provenance.
  - The output bytes MUST be `JSON.stringify(obj, null, 2) + "\\n"` (force `\\n` newlines; no locale formatting).

  **Determinism (HARD)**  
  - Generator MUST enforce stable ordering:
    - `inputs` sorted by `path` (ascending).
    - `task_board.entries` sorted by `wp_id` (ascending).
    - `traceability.mappings` sorted by `base_wp_id` (ascending).
    - `signatures.consumed` sorted by `signature` (ascending).
    - `gates.validator.wp_gate_summaries` sorted by `wp_id` (ascending) if present.
  - Generator MUST avoid locale/time dependent formatting (no wall clock calls).

  **Minimum schema (normative)**  
  ProductGovernanceSnapshot
  - schema_version: "hsk.product_governance_snapshot@0.1"
  - spec: { spec_target: string, spec_sha1: string }
  - git: { head_sha?: string } (generator SHOULD default to `git: {}`; omit head_sha unless explicitly enabled)
  - inputs: [{ path: string, sha256: string }]
  - task_board: { entries: [{ wp_id: string, status_token: string }] }
  - traceability: { mappings: [{ base_wp_id: string, active_packet_path: string }] }
  - signatures: { consumed: [{ signature: string, purpose: string, wp_id?: string }] }
  - gates: { orchestrator: { last_refinement?: string, last_signature?: string, last_prepare?: string }, validator: { wp_gate_summaries?: [{ wp_id: string, verdict?: string, status?: string, gates_passed?: string[] }] } }
    - `wp_gate_summaries` MUST be a list (not a map/object) and MUST omit timestamps and raw logs/bodies.

  **Security (HARD)**  
  - Snapshot MUST NOT include secrets, environment variables, or raw Role Mailbox message bodies.
  - References to external artifacts MUST be by hash/ref only.

  **Command surface (HARD)**  
  - A single deterministic command MUST exist to generate/refresh the snapshot (e.g., `just governance-snapshot`).
  - A validator MUST exist to check schema + determinism + leak-safety (e.g., `just validator-governance-snapshot`).
  ```
