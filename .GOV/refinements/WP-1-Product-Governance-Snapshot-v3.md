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
- WP_ID: WP-1-Product-Governance-Snapshot-v3
- CREATED_AT: 2026-02-06T06:58:49.221Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.125.md
- SPEC_TARGET_SHA1: d16eb1eb5045e858112b2ce477f27aa0200621b0
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja060220260923
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Product-Governance-Snapshot-v3

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- NONE. Handshake_Master_Spec_v02.125.md Section 7.5.4.10 defines the Product Governance Snapshot artifact, canonical input whitelist, deterministic output requirements, minimum schema (including list-based validator summaries), and security constraints required to implement and validate mechanically.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- NONE required by 7.5.4.10.
- OPTIONAL (future): define a dedicated Flight Recorder event family for snapshot generation/validation (out of scope for this WP unless explicitly added to the packet).

### RED_TEAM_ADVISORY (security failure modes)
- Leak risk: snapshot MUST NOT include secrets/PII, environment variables, or raw Role Mailbox message bodies; hashes/refs only.
- Path traversal risk: generator MUST restrict reads to the canonical whitelist only (and the resolved spec file named by SPEC_CURRENT).
- Tampering/spoofing risk: snapshot MUST include sha256(file_bytes) for each input; output MAY include git HEAD sha only behind an explicit flag.
- Nondeterminism risk: avoid wall-clock timestamps and locale-dependent formatting; enforce stable ordering and canonical JSON output bytes.

### PRIMITIVES (traits/structs/enums)
- ProductGovernanceSnapshot JSON schema and sub-objects (per 7.5.4.10).
- Deterministic hashing helper (sha256 over file bytes).
- Canonical ordering rule: stable sort by path; stable JSON formatting.
- Parsers (deterministic):
  - TASK_BOARD entries: `- **[WP_ID]** - [TOKEN]` (ignore trailing text).
  - WP_TRACEABILITY_REGISTRY table: `{ base_wp_id, active_packet_path }`.
  - SIGNATURE_AUDIT table: `{ signature, purpose, wp_id? }` (no timestamps).
  - ORCHESTRATOR_GATES.json: last_refinement/last_signature/last_prepare from last entries by array order (ignore timestamps).
  - validator_gates summaries: list entries only (no timestamps; no raw logs).

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Handshake_Master_Spec_v02.125.md Section 7.5.4.10 defines input whitelist, output location, minimum schema, determinism constraints, security constraints, and required command surface/validator.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Master Spec v02.125 includes a normative Section 7.5.4.10 covering snapshot definition, schema, determinism, and security constraints.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.125.md 7.5.4.10 Product Governance Snapshot (HARD)
- CONTEXT_START_LINE: 42852
- CONTEXT_END_LINE: 42892
- CONTEXT_TOKEN: #### 7.5.4.10 Product Governance Snapshot (HARD)
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

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.125.md 7.5.4.3 Canonical governance artifacts (kernel)
- CONTEXT_START_LINE: 28377
- CONTEXT_END_LINE: 28390
- CONTEXT_TOKEN: #### 7.5.4.3 Canonical governance artifacts (kernel)
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 7.5.4.3 Canonical governance artifacts (kernel)

  Kernel objective: a fresh agent can reconstruct \\u00e2\\u20ac\\u0153what is true\\u00e2\\u20ac\\u009d by opening a small stable set of files.

  Required artifacts (canonical locations):
  - `.GOV/roles_shared/SPEC_CURRENT.md`: single pointer to the current authoritative Master Spec.
  - `.GOV/roles_shared/TASK_BOARD.md`: global execution state SSoT (minimal entries; details live in packets).
  - `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`: Base WP \\u00e2\\u2020\\u2019 Active Packet mapping (prevents revision ambiguity).
  - `.GOV/refinements/<WP_ID>.md`: Technical Refinement Block artifact (ASCII-only; spec anchors; enrichment decision; approval evidence).
  - `.GOV/task_packets/stubs/<WP_ID>.md`: non-executable backlog stub (no signature).
  - `.GOV/task_packets/<WP_ID>.md`: executable task contract (ASCII-only; required headings; validation manifests).
  - `.GOV/roles_shared/SIGNATURE_AUDIT.md`: append-only signature log.
  - `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json`, `.GOV/validator_gates/{WP_ID}.json`: gate state (deterministic JSON).
  - `.GOV/templates/`: canonical templates for stubs/refinements/packets (prevents drift).
  ```
