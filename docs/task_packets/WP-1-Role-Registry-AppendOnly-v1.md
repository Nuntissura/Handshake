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
  - src/backend/handshake_core/src/ai_ready_data/records.rs
  - src/backend/handshake_core/src/api/flight_recorder.rs
  - src/backend/handshake_core/src/lib.rs
  - src/backend/handshake_core/src/ace/validators/mod.rs
  - src/backend/handshake_core/src/ace/validators/role_registry_append_only.rs
  - src/backend/handshake_core/src/mex/conformance.rs
  - src/backend/handshake_core/src/workflows.rs
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
    - `AtelierRoleSpec` required keys (spec Section 6.3.3.5.7.1):
      - `role_id: string` (required; stable; never reused)
      - `department_id: string` (required)
      - `display_name: string` (required)
      - `modes_supported: (\"representational\"|\"conceptual\")[]` (required; subset)
      - `content_profiles_supported: string[]` (required)
      - `claim_features: string[]` (required)
      - `extract_contracts: RoleContractSpec[]` (required)
      - `produce_contracts: RoleContractSpec[]` (required)
      - `allowed_models: string[]` (required)
      - `allowed_tools: string[]` (required)
      - `vocab_namespace: string` (required; may be empty string)
      - `proposal_policy: \"disabled\"|\"queue_only\"|\"auto_accept_with_threshold\"` (required)
    - `AtelierRoleSpec` OPTIONAL extensions (NOT required by Section 6.3.3.5.7.1):
      - `aliases?: string[]` (optional; anchored to Addendum: 3.3 \"Renames are aliases; role_id does not change\")
      - `deprecated?: boolean` (optional; anchored to Addendum: 3.3 \"existing roles MAY be deprecated (explicitly)\")
    - `RoleContractSpec { contract_id: string, schema_json: object }`
    - Contract id format (required per spec Section 6.3.3.5.7.1):
      - Extraction: `ROLE:<role_id>:X:<ver>`
      - Compose: `ROLE:<role_id>:C:<ver>`
      - Any contract surface change requires a new `<ver>` (new `contract_id`).
  - Build/CI validator (Node): `scripts/validation/atelier_role_registry_check.mjs`
    - Baseline strategy (locked): load baseline RolePack bytes from git:
      - primary: `git show main:assets/atelier_rolepack_digital_production_studio_v1.json`
      - fallback: `git show origin/main:assets/atelier_rolepack_digital_production_studio_v1.json`
      - if neither exists: baseline is treated as empty (first publish), but internal invariants still apply
    - Reads RolePack JSON and enforces:
      - role_id set is append-only vs baseline
      - contract_id -> ContractSurfaceHash is immutable once published
      - role_id uniqueness (no duplicates within a pack)
      - contract_id format parse (ROLE:<role_id>:(X|C):<ver>)
  - Runtime validator (Rust): `src/backend/handshake_core/src/ace/validators/role_registry_append_only.rs`
    - `RoleId(String)` and `DepartmentId(String)` newtypes
    - `RoleContractKind { Extract, Produce }`
    - `RoleSpecEntry { role_id: RoleId, department_id: DepartmentId, display_name: String, aliases: Vec<RoleId> }` (aliases default empty; populated from OPTIONAL Addendum 3.3 extension when present)
    - `ContractSurfaceHash([u8; 32])` (sha256)
    - `RoleContractSurface { contract_id: String, role_id: RoleId, kind: RoleContractKind, version: String, schema_hash: ContractSurfaceHash }`
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
- (Mechanical manifest for audit. This section records the "What" (hashes/lines) for the Validator's "How/Why" audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`docs/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- Range validated (for Pre-SHA1): `7e13c5c37823ea94b0cc7306383820c0567f0d08..HEAD`
- **Target File**: `assets/atelier_rolepack_digital_production_studio_v1.json`
- **Start**: 1
- **End**: 1907
- **Line Delta**: 1907
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `17d4468ef1776317c29d9b0875a467cf5e8287de`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `scripts/validation/atelier_role_registry_check.mjs`
- **Start**: 1
- **End**: 240
- **Line Delta**: 240
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `e95294a6f27b0bced25df5667c9a0f2fbc29150d`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `scripts/validation/codex-check.mjs`
- **Start**: 67
- **End**: 115
- **Line Delta**: 3
- **Pre-SHA1**: `e59e79824cfda7034590bad79f061fc71aa053dd`
- **Post-SHA1**: `e56343c938b34afe52095ca6026fca0aebffc8e9`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/ace/validators/mod.rs`
- **Start**: 37
- **End**: 38
- **Line Delta**: 1
- **Pre-SHA1**: `a12fb18b983b0aee923c442bc522912b38fb314d`
- **Post-SHA1**: `8ded388435c549b63390171146faefd2bcd79d4b`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/ace/validators/role_registry_append_only.rs`
- **Start**: 1
- **End**: 434
- **Line Delta**: 434
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `348f593fb41d7a80cbd835b6a7e7b2195c3ea738`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/ai_ready_data/records.rs`
- **Start**: 24
- **End**: 141
- **Line Delta**: 8
- **Pre-SHA1**: `8fba3e073310111acabf51d77c2926a3962b283f`
- **Post-SHA1**: `d74e7089e7b718c487da832abca31837bdfedb83`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/api/flight_recorder.rs`
- **Start**: 1
- **End**: 237
- **Line Delta**: 42
- **Pre-SHA1**: `ea1346b4984a634573fbc011744b69460b2899db`
- **Post-SHA1**: `787ad782b29bcb62be07a75b565feecfd2269cfe`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/mex/conformance.rs`
- **Start**: 333
- **End**: 337
- **Line Delta**: 0
- **Pre-SHA1**: `3119e894c48280e29ece86a94eef93a5edaa80ab`
- **Post-SHA1**: `056494c5fcfce2aefe301b803ee2bf05897c4914`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/workflows.rs`
- **Start**: 2808
- **End**: 3903
- **Line Delta**: 2
- **Pre-SHA1**: `e6528eb704a36dd5b817e75c83213370bff3a4b4`
- **Post-SHA1**: `86cc7746ad4c75429b6b8cfd7351b92b71e8d159`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/tests/role_registry_append_only_tests.rs`
- **Start**: 1
- **End**: 221
- **Line Delta**: 221
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `6411ebe7c38ea12fd220d99e64b90f459fedfae3`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: In Progress
- What changed in this update: Bootstrapped (claimed WP; starting work)
- Next step / handoff hint: Draft SKELETON (types/interfaces only) and await Validator approval

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
