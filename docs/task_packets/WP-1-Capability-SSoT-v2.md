# Task Packet: WP-1-Capability-SSoT-v2

## METADATA
- TASK_ID: WP-1-Capability-SSoT-v2
- WP_ID: WP-1-Capability-SSoT-v2
- BASE_WP_ID: WP-1-Capability-SSoT (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-01-18T15:01:56.223Z
- REQUESTOR: ilja
- AGENT_ID: codex-cli
- ROLE: Orchestrator
- CODER_MODEL: GPT-5.2 (Codex CLI)
- CODER_REASONING_STRENGTH: HIGH
- **Status:** In Progress
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja180120261552

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-Capability-SSoT-v2.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Remediate capability SSoT enforcement + capability-check audit logging to align with Master Spec v02.113 Section 11.1 ([HSK-4001]), including the 11.1.6 capability registry artifact workflow (draft/validate/diff/review/publish), and clear the prior revalidation FAIL drivers (COR-701 manifest mismatch and audit-field drift).
- Why: Capability checks are a security boundary; incorrect UnknownCapability handling and/or missing audit fields breaks governance invariants and blocks Phase-1 gates (pre-work/post-work).
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/capabilities.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/terminal/mod.rs
  - src/backend/handshake_core/src/terminal/guards.rs
  - src/backend/handshake_core/src/mex/runtime.rs
  - src/backend/handshake_core/src/mex/gates.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/bin/capability_registry_workflow.rs
  - src/backend/handshake_core/schemas/capability_registry.schema.json
  - src/backend/handshake_core/Cargo.toml
  - src/backend/handshake_core/Cargo.lock
  - capability_registry_draft.json
  - capability_registry_diff.json
  - capability_registry_review.json
  - assets/capability_registry.json
- OUT_OF_SCOPE:
  - Top-level directory creation other than `assets/` (Codex [CX-106])
  - Changes to Master Spec or Codex/protocol files
  - Frontend/UI changes (app/)
  - Unrelated workflow/job kinds not required for this WP's DONE_MEANS

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- [CX-106 AUTHORIZATION] 2026-01-18: Operator authorized top-level `assets/` directory + capability registry artifact files for WP-1-Capability-SSoT-v2 (evidence: "AUTHORIZE top-level assets/ dir + capability registry artifact files for WP-1-Capability-SSoT-v2").
- WAIVER-SCOPE-EXPAND-WP-1-Capability-SSoT-v2-001 [CX-573F]
  - Date: 2026-01-18
  - Scope: Expand IN_SCOPE_PATHS beyond this packet as needed to satisfy DONE_MEANS (incl. additional capability-check call sites across the codebase and any supporting files).
  - Justification: Operator explicitly waived out-of-scope gating to unblock implementation and complete full "every capability check" audit-field alignment.
  - Approver: Operator (chat waiver: "i waive out of scope" / "i waive the scope, it is allowed")
  - Expiry: On WP closure (validation complete).

- WAIVER-SCOPE-EXPAND-WP-1-Capability-SSoT-v2-002 [CX-573F SCOPE-EXPAND]
  - Date: 2026-01-18
  - Scope: Explicitly expand capability-check call site coverage (Terminal + MEX) to satisfy "Every capability check (Allow or Deny) MUST be recorded" (Master Spec v02.113 11.1 [HSK-4001] Audit Requirement). Minimum call sites: src/backend/handshake_core/src/terminal/mod.rs, src/backend/handshake_core/src/terminal/guards.rs, src/backend/handshake_core/src/mex/runtime.rs, src/backend/handshake_core/src/mex/gates.rs.
  - Justification: Validator requested scope alignment so the WP can satisfy the spec's global capability-check audit invariant across all runtime gates, not only the registry core.
  - Approver: Operator (instruction in chat to expand IN_SCOPE_PATHS and record this waiver)
  - Expiry: End of Phase 1

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Capability-SSoT-v2

# Capability registry artifacts (Master Spec v02.113 11.1.6)
node -e "JSON.parse(require('fs').readFileSync('capability_registry_draft.json','utf8')); console.log('draft: ok')"
node -e "JSON.parse(require('fs').readFileSync('capability_registry_diff.json','utf8')); console.log('diff: ok')"
node -e "JSON.parse(require('fs').readFileSync('capability_registry_review.json','utf8')); console.log('review: ok')"
node -e "JSON.parse(require('fs').readFileSync('assets/capability_registry.json','utf8')); console.log('publish: ok')"

# Targeted unit tests
cd src/backend/handshake_core
cargo test capabilities
cargo test workflows

# Full deterministic gates
cd ../..
just cargo-clean
just post-work WP-1-Capability-SSoT-v2
```

### DONE_MEANS
- `just pre-work WP-1-Capability-SSoT-v2` passes.
- `just post-work WP-1-Capability-SSoT-v2` passes (no COR-701 manifest mismatches).
- Unknown capability IDs are rejected with error code `HSK-4001: UnknownCapability` (Master Spec 11.1 [HSK-4001]).
- Every capability check (allow/deny) emits a Flight Recorder event capturing: `capability_id`, `actor_id`, `job_id` (if applicable), and `decision_outcome` (Master Spec 11.1 audit requirement).
- Capability registry workflow artifacts exist and are valid JSON per Master Spec 11.1.6: `capability_registry_draft.json`, `capability_registry_diff.json`, `capability_registry_review.json`, and `assets/capability_registry.json`.
- Deterministic manifest entries exist for every changed non-doc file (correct Start/End/LineDelta/PreSHA1/PostSHA1).

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.113.md (recorded_at: 2026-01-18T15:01:56.223Z)
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md 11.1 ([HSK-4001]), 11.1.3.1-11.1.3.2, 11.1.6
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md
- WP Traceability: docs/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- Prior packets for BASE_WP_ID:
  - docs/task_packets/WP-1-Capability-SSoT.md (superseded; revalidation FAIL due to COR-701 manifest mismatch + spec drift)
- Preserved:
  - CapabilityRegistry SSoT concept and UnknownCapability rejection invariant.
  - Axis inheritance behavior (axis-only grant allows axis:scope) per 11.1.3.1.
- Changed / added in this revision:
  - Re-anchor to Master Spec v02.113 (11.1) and require audit-field alignment (`decision_outcome` + actor_id semantics).
  - Include Master Spec v02.113 11.1.6 capability registry artifact workflow (draft/diff/review/publish).
  - Replace prior packet's failing deterministic manifest with a fresh COR-701 manifest that matches the actual code state post-fix.

## BOOTSTRAP
- FILES_TO_OPEN:
  - docs/START_HERE.md
  - docs/SPEC_CURRENT.md
  - Handshake_Master_Spec_v02.113.md
  - docs/task_packets/WP-1-Capability-SSoT.md
  - docs/refinements/WP-1-Capability-SSoT-v2.md
  - docs/task_packets/WP-1-Capability-SSoT-v2.md
  - src/backend/handshake_core/src/capabilities.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/storage/mod.rs
- SEARCH_TERMS:
  - "HSK-4001"
  - "UnknownCapability"
  - "CapabilityRegistry"
  - "can_perform"
  - "is_valid"
  - "log_capability_check"
  - "decision_outcome"
  - "\"capability_id\""
  - "\"outcome\""
  - ".with_actor_id("
  - "capability_profile_id"
  - "required_capabilities_for_job"
  - "profile_can"
- RUN_COMMANDS:
  ```bash
  just pre-work WP-1-Capability-SSoT-v2
  cd src/backend/handshake_core
  cargo test capabilities
  cargo test workflows
  cd ../..
  just cargo-clean
  just post-work WP-1-Capability-SSoT-v2
  ```
- RISK_MAP:
  - "unknown capability accepted" -> "security boundary bypass"
  - "audit fields drift" -> "cannot prove allow/deny decisions; governance failure"
  - "actor_id misuse" -> "misattribution in Flight Recorder analysis"
  - "manifest mismatch" -> "post-work gate failure; WP cannot validate"
  - "job/profile mapping regression" -> "job creation fails or over-privileges jobs"

## SKELETON
- Proposed interfaces/types/contracts:
- `RegistryError::UnknownCapability` MUST surface the code string `HSK-4001: UnknownCapability` in its error.
- Capability check logging MUST emit `decision_outcome` (not just `outcome`) and must not overload Flight Recorder `actor_id` with a capability profile identifier.
- Open questions:
- Should `actor_id` for capability checks be left as the default (`agent`) or set to a stable component ID (e.g., `workflow_engine`) to match "System Component ID" semantics?
- Notes:
- Keep packet ASCII-only and ensure COR-701 manifests are captured after staging (use `just cor701-sha`).

## IMPLEMENTATION
- CapabilityRegistry contract alignment (Master Spec 11.1.3.2):
  - `CapabilityRegistry::is_valid(&self, capability_id: &str) -> bool`
  - `CapabilityRegistry::can_perform(&self, requested: &str, granted: &[String]) -> bool`
  - `CapabilityRegistry::enforce_can_perform(&self, requested: &str, granted: &[String]) -> Result<bool, RegistryError>`
- Enforcement boundary behavior:
  - Unknown capability -> `Err(RegistryError::UnknownCapability(..))` with substring `HSK-4001: UnknownCapability` and audit event logged.
  - Known-but-denied -> `Ok(false)` and audit event logged.
  - Allowed -> `Ok(true)` and audit event logged.
- Capability-check Flight Recorder payload standardization:
  - Keys exactly: `capability_id`, `actor_id`, `job_id`, `decision_outcome`
  - Actor ID semantics: stable component IDs; do not overload with capability_profile_id.
- Spec 11.1.6 capability registry workflow:
  - Input: `src/backend/handshake_core/mechanical_engines.json`
  - JSON Schema: `src/backend/handshake_core/schemas/capability_registry.schema.json`
  - Workflow binary: `src/backend/handshake_core/src/bin/capability_registry_workflow.rs`
  - Artifacts: `capability_registry_draft.json`, `capability_registry_diff.json`, `capability_registry_review.json`, `assets/capability_registry.json`

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE. Keep packet ASCII-only.)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`docs/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.

- **Target File**: `assets/capability_registry.json`
- **Start**: 1
- **End**: 438
- **Line Delta**: 438
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `3608110366ffd1b93944aaf2f5232a319e755cf4`
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

- **Target File**: `capability_registry_diff.json`
- **Start**: 1
- **End**: 40
- **Line Delta**: 40
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `821008bc027a685740d1d7fe5da8592166de2afd`
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

- **Target File**: `capability_registry_draft.json`
- **Start**: 1
- **End**: 438
- **Line Delta**: 438
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `3608110366ffd1b93944aaf2f5232a319e755cf4`
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

- **Target File**: `capability_registry_review.json`
- **Start**: 1
- **End**: 6
- **Line Delta**: 6
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `73da10ea1bf1b619f1d316e9615c1fef4214d8c1`
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

- **Target File**: `src/backend/handshake_core/Cargo.lock`
- **Start**: 1
- **End**: 4709
- **Line Delta**: 356
- **Pre-SHA1**: `5593f6381e5a819fd9dc599780be0e9a52ffff7a`
- **Post-SHA1**: `07527f4c3632d635cdcedba6e21485c17b6a5f65`
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

- **Target File**: `src/backend/handshake_core/Cargo.toml`
- **Start**: 1
- **End**: 40
- **Line Delta**: 1
- **Pre-SHA1**: `e437bd6391dc446bf9e578e23bc55394382778ec`
- **Post-SHA1**: `8ca028a9122498ab485cb584896c5b307e969df5`
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

- **Target File**: `src/backend/handshake_core/schemas/capability_registry.schema.json`
- **Start**: 1
- **End**: 49
- **Line Delta**: 49
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `2ab749611e1cf362d6c061a557c94d0d9cdcdd04`
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

- **Target File**: `src/backend/handshake_core/src/bin/capability_registry_workflow.rs`
- **Start**: 1
- **End**: 644
- **Line Delta**: 644
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `d3355868b0a5a0ef5a8d5edf186b6592352a6c96`
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

- **Target File**: `src/backend/handshake_core/src/capabilities.rs`
- **Start**: 1
- **End**: 437
- **Line Delta**: 13
- **Pre-SHA1**: `50ce284d3463aff3b2b3f80042038ef2cb3755b6`
- **Post-SHA1**: `17a8cc61f381a5865cb2eefe686043a2ed65b34d`
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

- **Target File**: `src/backend/handshake_core/src/flight_recorder/mod.rs`
- **Start**: 1
- **End**: 1510
- **Line Delta**: 2
- **Pre-SHA1**: `57ea7d1edf5dd69e4623f6222b96e60ccd744188`
- **Post-SHA1**: `57c902d88e89c18c6668fac6d445e09952af298c`
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

- **Target File**: `src/backend/handshake_core/src/mex/gates.rs`
- **Start**: 1
- **End**: 306
- **Line Delta**: 0
- **Pre-SHA1**: `aa364b977e63676a5c62c88537c6ca317ad1c294`
- **Post-SHA1**: `fbb704f93abfba479e68f7169e4be5719d66c009`
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

- **Target File**: `src/backend/handshake_core/src/mex/runtime.rs`
- **Start**: 1
- **End**: 323
- **Line Delta**: 1
- **Pre-SHA1**: `4d7e7c3d06d0cbe750a57a2123841c7b5b739e1d`
- **Post-SHA1**: `3e3b7378719cd57a466108254c270557ca0ffc01`
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

- **Target File**: `src/backend/handshake_core/src/terminal/guards.rs`
- **Start**: 1
- **End**: 227
- **Line Delta**: 0
- **Pre-SHA1**: `a409e8ecb4b96fbfc52cb7fbddb7234740c6fde2`
- **Post-SHA1**: `226731dc6e4e8cd7b38c12730dd17ef67bcefdf3`
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

- **Target File**: `src/backend/handshake_core/src/terminal/mod.rs`
- **Start**: 1
- **End**: 705
- **Line Delta**: -8
- **Pre-SHA1**: `b3c681167aa6b94c13f00f71c545ba9eefd4980e`
- **Post-SHA1**: `0392bee8d3722eaae81cdf58d333df827cf8a02f`
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
- **Start**: 1
- **End**: 1636
- **Line Delta**: -48
- **Pre-SHA1**: `c521e4dc8878f7fa8f51b56b47ba2290b7c290e1`
- **Post-SHA1**: `50b554e84726d764c8922683329de49548e711a6`
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

- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.113.md

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: In Progress
- What changed in this update: Implemented capability registry enforcement + audit logging standardization and added deterministic manifest entries for all changed non-doc files.
- Next step / handoff hint: Run `just post-work WP-1-Capability-SSoT-v2` and proceed to commit once the gate is clean.

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
