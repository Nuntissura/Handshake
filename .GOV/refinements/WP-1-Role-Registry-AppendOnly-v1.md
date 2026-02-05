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
- WP_ID: WP-1-Role-Registry-AppendOnly-v1
- CREATED_AT: 2026-01-30T00:00:00Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.123.md
- SPEC_TARGET_SHA1: 4d406dcc1a75570d2f17659e0ac40d68a22f211a
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja300120262137
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Role-Registry-AppendOnly-v1

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- NONE. The Master Spec defines: (1) role_id stability (never reused), (2) an append-only role registry requirement, and (3) a hard validator requirement to fail builds if a previously-declared role_id disappears or a role contract surface changes without versioning.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- On detection of an append-only violation (role_id removed, or contract surface drift without version bump), the failing validation MUST surface a Diagnostic and (when running inside an app job/workflow context) emit FR-EVT-003 (DiagnosticEvent) referencing the created Diagnostic.id (no duplication of full payload in Flight Recorder).
- Correlate the DiagnosticEvent to the originating job/workflow (job_id/workflow context) so Operator Consoles can deep-link Job -> Diagnostic/Problem.

### RED_TEAM_ADVISORY (security failure modes)
- Validator bypass: role registry changes can evade enforcement if any runtime path loads roles from an unvalidated source; require a single canonical registry source and ensure validators read the same source.
- Role ID reuse/collision: reusing a role_id (or mapping two roles to one role_id) breaks determinism and audit; enforce unique role_id and never-reuse.
- Silent contract drift: editing schema_json/spec_json for an existing contract_id without bumping contract version undermines replayability; enforce append-only-by-contract_id (contract_id -> schema_hash is immutable).
- Non-canonical hashing: unstable JSON serialization/order can cause spurious diffs or missed drift; require canonical JSON serialization and stable ordering for hashing.

### PRIMITIVES (traits/structs/enums)
- Role identity: RoleId (opaque string).
- Registry snapshot model: RoleRegistrySnapshot, RoleSpecEntry { role_id, department_id, display_name, ... }, RoleAlias { alias, role_id }.
- Contract surface model: RoleContractSurface { contract_id, kind, version, schema_hash } and a stable ContractSurfaceHash (sha256 over canonical bytes).
- Validator: RoleRegistryAppendOnlyValidator that checks role_id presence is monotonic and contract_id/schema_hash are immutable once published.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: The Master Spec explicitly requires (a) stable role_id, (b) append-only registry semantics, and (c) a blocking validator on removal or silent contract surface changes.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The Master Spec already specifies the required invariants (stable role_id, append-only registry, validator fail conditions) and the DiagnosticEvent linkability pattern; remaining choices (file layout, canonicalization strategy, exact validator wiring points) are implementation details.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.123.md Addendum: 3.3 Lossless role catalog + append-only registry (HARD)
- CONTEXT_START_LINE: 25399
- CONTEXT_END_LINE: 25409
- CONTEXT_TOKEN: append-only
- EXCERPT_ASCII_ESCAPED:
  ```text
  Addendum: 3.3 Lossless role catalog + append-only registry (HARD)

  - Role identifiers (role_id) are stable; renames are aliases; role_id does not change.
  - The role registry is append-only: new roles may be added; roles may be deprecated; roles must not be removed.
  - Validators must fail any build that removes a previously declared role_id or silently changes a role contract surface.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.123.md 6.3.3.5.7.1 Role Registry + Dual Contracts (AtelierRoleSpec)
- CONTEXT_START_LINE: 25071
- CONTEXT_END_LINE: 25087
- CONTEXT_TOKEN: AtelierRoleSpec
- EXCERPT_ASCII_ESCAPED:
  ```text
  6.3.3.5.7.1 Role Registry + Dual Contracts

  Config entity: AtelierRoleSpec (versioned) includes role_id (stable; never reused),
  claim_features, extract_contracts (ROLE:<role_id>:X:<ver>), and produce_contracts
  (ROLE:<role_id>:C:<ver>).
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.123.md 11.5.1 FR-EVT-003 (DiagnosticEvent)
- CONTEXT_START_LINE: 57637
- CONTEXT_END_LINE: 57649
- CONTEXT_TOKEN: type: 'diagnostic';
- EXCERPT_ASCII_ESCAPED:
  ```text
  FR-EVT-003 (DiagnosticEvent)

  A DiagnosticEvent links a Flight Recorder trace to a Diagnostic (Diagnostic.id) without
  duplicating the full Diagnostic payload. type: 'diagnostic'; diagnostic_id: string.
  ```

