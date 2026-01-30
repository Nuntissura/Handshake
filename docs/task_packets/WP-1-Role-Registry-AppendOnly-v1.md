# Task Packet: WP-1-Role-Registry-AppendOnly-v1

## METADATA
- TASK_ID: WP-1-Role-Registry-AppendOnly-v1
- WP_ID: WP-1-Role-Registry-AppendOnly-v1
- BASE_WP_ID: WP-1-Role-Registry-AppendOnly (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-01-30T20:58:04.964Z
- REQUESTOR: Operator (ilja)
- AGENT_ID: CodexCLI-GPT-5.2 (Orchestrator)
- ROLE: Orchestrator
- CODER_MODEL: GPT-5.2
- CODER_REASONING_STRENGTH: EXTRA_HIGH
- **Status:** In Progress
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja300120262137

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-Role-Registry-AppendOnly-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Implement an append-only Role Registry (Atelier/Lens RolePack) with stable role_id semantics, plus a blocking validator that fails when a previously declared role_id disappears or a role contract surface changes without an explicit version/contract id bump.
- Why: Prevent silent drift (lost roles / reused ids / silent contract changes) that breaks determinism, auditability, and replayability for Atelier/Lens role passes and role-lane retrieval.
- IN_SCOPE_PATHS:
  - docs/task_packets/WP-1-Role-Registry-AppendOnly-v1.md
  - docs/refinements/WP-1-Role-Registry-AppendOnly-v1.md
  - docs/WP_TRACEABILITY_REGISTRY.md
  - docs/TASK_BOARD.md
  - assets/atelier_rolepack_digital_production_studio_v1.json
  - scripts/validation/atelier_role_registry_check.mjs
  - scripts/validation/codex-check.mjs
  - src/backend/handshake_core/src/lib.rs
  - src/backend/handshake_core/src/ace/validators/mod.rs
  - src/backend/handshake_core/src/ace/validators/role_registry_append_only.rs
  - src/backend/handshake_core/tests/role_registry_append_only_tests.rs
- OUT_OF_SCOPE:
  - Expanding the role catalog beyond the Master Spec RolePack inventory (roles are defined by spec; this WP enforces drift controls).
  - Multi-workspace / multi-user role registry merge and sync (Phase 2+).
  - Implementing Atelier/Lens extraction runtime itself (separate WPs; this WP focuses on role registry drift enforcement).

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Role-Registry-AppendOnly-v1

# Hygiene / CI parity:
just lint
just test
just validator-spec-regression
just validator-scan

just post-work WP-1-Role-Registry-AppendOnly-v1
```

### DONE_MEANS
- Role registry source is present (RolePack or equivalent), and role_id entries are stable (no reuse) and uniquely identified.
- Append-only enforcement is implemented: removing a previously declared role_id causes a deterministic, blocking failure (validator/CI).
- Contract surface drift enforcement is implemented: changing an existing contract surface without a version/contract id bump causes a deterministic, blocking failure.
- `just pre-work WP-1-Role-Registry-AppendOnly-v1` and `just post-work WP-1-Role-Registry-AppendOnly-v1` pass on the WP branch worktree.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.123.md (recorded_at: 2026-01-30T20:58:04.964Z)
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.123.md Addendum: 3.3 (Lossless role catalog + append-only registry) + 6.3.3.5.7.1 (AtelierRoleSpec) + 6.3.3.5.7.23 / 12 (Role registry: Digital Production Studio RolePack)
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md
- WP Traceability: docs/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- N/A (first activated packet for BASE_WP_ID; prior artifact is a non-executable stub: docs/task_packets/stubs/WP-1-Role-Registry-AppendOnly-v1.md).

## BOOTSTRAP
- FILES_TO_OPEN:
  - docs/task_packets/WP-1-Role-Registry-AppendOnly-v1.md
  - docs/task_packets/stubs/WP-1-Role-Registry-AppendOnly-v1.md
  - docs/refinements/WP-1-Role-Registry-AppendOnly-v1.md
  - docs/WP_TRACEABILITY_REGISTRY.md
  - docs/TASK_BOARD.md
  - Handshake_Master_Spec_v02.123.md
  - scripts/validation/wp-activation-traceability-check.mjs
- SEARCH_TERMS:
  - "Lossless role catalog"
  - "append-only registry"
  - "AtelierRoleSpec"
  - "role_id"
  - "contract_id"
- RUN_COMMANDS:
  ```bash
  rg -n "Lossless role catalog|append-only registry|AtelierRoleSpec|role_id" Handshake_Master_Spec_v02.123.md
  just pre-work WP-1-Role-Registry-AppendOnly-v1
  ```
- RISK_MAP:
  - "False-positive drift failures from non-canonical hashing" -> "build/CI blocking"
  - "Silent role_id reuse/alias collision" -> "broken auditability and non-replayable lanes"

## SKELETON
- Proposed interfaces/types/contracts:
  - RolePack source (JSON): `assets/atelier_rolepack_digital_production_studio_v1.json`
    - `AtelierRolePack { pack_id: string, pack_version: string, spec_id: string, roles: AtelierRoleSpec[] }`
    - `AtelierRoleSpec` (required keys per spec §6.3.3.5.7.1; fields are required but may allow empty list/string where appropriate):
      - `role_id: string` (required; stable; never reused; lower_snake_case `[a-z0-9_]{1,64}`)
      - `department_id: string` (required; lower_snake_case `[a-z0-9_]{1,64}`)
      - `display_name: string` (required)
      - `modes_supported: (\"representational\"|\"conceptual\")[]` (required; subset; must be non-empty)
      - `content_profiles_supported: string[]` (required)
      - `claim_features: string[]` (required)
      - `extract_contracts: RoleContractSpec[]` (required)
      - `produce_contracts: RoleContractSpec[]` (required)
      - `allowed_models: string[]` (required)
      - `allowed_tools: string[]` (required)
      - `vocab_namespace: string` (required; may be empty string)
      - `proposal_policy: \"disabled\"|\"queue_only\"|\"auto_accept_with_threshold\"` (required)
      - `aliases: string[]` (required; rename aliases; may be empty; each alias must match `role_id` format)
      - `lifecycle: \"active\"|\"deprecated\"` (required; explicit deprecation allowed; removal forbidden)
    - `RoleContractSpec { contract_id: string, schema_json: object }`
    - Contract id format (required per spec §6.3.3.5.7.1):
      - Extraction: `ROLE:<role_id>:X:<ver>`
      - Compose: `ROLE:<role_id>:C:<ver>`
      - `<ver>` is a positive integer (u32). Any contract surface change requires a new `<ver>` (new `contract_id`).
  - Build/CI validator (Node): `scripts/validation/atelier_role_registry_check.mjs`
    - Baseline strategy (locked): load baseline RolePack bytes from git:
      - primary: `git show main:assets/atelier_rolepack_digital_production_studio_v1.json`
      - fallback: `git show origin/main:assets/atelier_rolepack_digital_production_studio_v1.json`
      - if neither exists: baseline is treated as empty (first publish), but internal invariants still apply
    - Reads RolePack JSON and enforces:
      - role_id set is append-only vs baseline
      - contract_id -> ContractSurfaceHash is immutable once published
      - role_id uniqueness + safe-id format
      - contract_id format + version parse (X/C kind + u32 `<ver>`)
  - Runtime validator (Rust): `src/backend/handshake_core/src/ace/validators/role_registry_append_only.rs`
    - `RoleId(String)` and `DepartmentId(String)` newtypes
    - `RoleContractKind { Extract, Produce }`
    - `RoleSpecEntry { role_id: RoleId, department_id: DepartmentId, display_name: String, aliases: Vec<String>, lifecycle: RoleLifecycle }`
    - `ContractSurfaceHash([u8; 32])` (sha256)
    - `RoleContractSurface { contract_id: String, role_id: RoleId, kind: RoleContractKind, version: u32, schema_hash: ContractSurfaceHash }`
    - Contract surface hashing (locked per refinement primitives):
      - `schema_hash = sha256(canonical_json_bytes(schema_json))`
      - canonical JSON bytes: stable key ordering + no whitespace variance (deterministic)
    - `RoleRegistrySnapshot { roles: Vec<RoleSpecEntry>, contracts: Vec<RoleContractSurface> }`
    - `RoleRegistryViolation` enum:
      - `RoleIdRemoved { role_id }`
      - `ContractSurfaceDrift { contract_id, expected_hash, got_hash }`
      - `DuplicateRoleId { role_id }`
      - `InvalidRoleId { role_id }`
    - Canonical hashing API (no logic yet): `fn canonical_json_sha256(value: &serde_json::Value) -> ContractSurfaceHash`
    - Validator API (no logic yet): `RoleRegistryAppendOnlyValidator::validate(current: &RoleRegistrySnapshot, baseline: &RoleRegistrySnapshot) -> Result<(), RoleRegistryViolation>`
    - Diagnostic + Flight Recorder requirement (locked per refinement):
      - On append-only violation / drift, record a `Diagnostic` via `DiagnosticsStore::record_diagnostic` (source=Validator) with job/workflow correlation fields when available.
      - DuckDB-backed diagnostics store (`DuckDbFlightRecorder`) emits FR-EVT-003 `FlightRecorderEventType::Diagnostic` with payload `{ diagnostic_id, wsid?, severity?, source? }` (no full diagnostic payload duplication).
  - Tests (Rust): `src/backend/handshake_core/tests/role_registry_append_only_tests.rs`
    - Coverage targets:
      - removing a previously published role_id fails
      - changing schema_json for an existing contract_id fails (unless contract_id/version bumps)
      - adding new role_id + new contract_id passes
      - canonical hashing is stable and deterministic
  - Hook (pre-commit): update `scripts/validation/codex-check.mjs` to run the role registry check (blocking)
- Open questions: NONE (decisions locked in this SKELETON)
- Notes:
  - IN_SCOPE_PATHS include files currently missing in this worktree and will be created during IMPLEMENTATION after SKELETON APPROVED:
    - `assets/atelier_rolepack_digital_production_studio_v1.json`
    - `scripts/validation/atelier_role_registry_check.mjs`
    - `src/backend/handshake_core/src/ace/validators/role_registry_append_only.rs`
    - `src/backend/handshake_core/tests/role_registry_append_only_tests.rs`

## IMPLEMENTATION
- (Coder fills after skeleton approval.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`docs/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `path/to/file`
- **Start**: <line>
- **End**: <line>
- **Line Delta**: <adds - dels>
- **Pre-SHA1**: `<hash>`
- **Post-SHA1**: `<hash>`
- **Gates Passed**:
  - [ ] anchors_present
  - [ ] window_matches_plan
  - [ ] rails_untouched_outside_window
  - [ ] filename_canonical_and_openable
  - [ ] pre_sha1_captured
  - [ ] post_sha1_captured
  - [ ] line_delta_equals_expected
  - [ ] all_links_resolvable
  - [ ] manifest_written_and_path_returned
  - [ ] current_file_matches_preimage
- **Lint Results**:
- **Artifacts**:
- **Timestamp**:
- **Operator**:
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_vXX.XX.md
- **Notes**:

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: In Progress
- What changed in this update: Bootstrapped (claimed WP; starting work)
- Next step / handoff hint: Draft SKELETON (types/interfaces only) and await Validator approval

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
