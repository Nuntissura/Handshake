# Task Packet: WP-1-Capability-SSoT-v2

## METADATA
- TASK_ID: WP-1-Capability-SSoT-v2
- WP_ID: WP-1-Capability-SSoT-v2
- BASE_WP_ID: WP-1-Capability-SSoT (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-01-18T15:01:56.223Z
- REQUESTOR: ilja
- AGENT_ID: codex-cli
- ROLE: Orchestrator
- CODER_MODEL: <unclaimed>
- CODER_REASONING_STRENGTH: <unclaimed> (LOW | MEDIUM | HIGH | EXTRA_HIGH)
- **Status:** Ready for Dev
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja180120261552

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-Capability-SSoT-v2.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Remediate capability SSoT enforcement + capability-check audit logging to align with Master Spec v02.113 Section 11.1 ([HSK-4001]), and clear the prior revalidation FAIL drivers (COR-701 manifest mismatch and audit-field drift).
- Why: Capability checks are a security boundary; incorrect UnknownCapability handling and/or missing audit fields breaks governance invariants and blocks Phase-1 gates (pre-work/post-work).
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/capabilities.rs
  - src/backend/handshake_core/src/workflows.rs
- OUT_OF_SCOPE:
  - Top-level directory creation without explicit Operator approval (Codex [CX-106])
  - Changes to Master Spec or Codex/protocol files
  - Frontend/UI changes (app/)
  - Unrelated workflow/job kinds not required for this WP's DONE_MEANS

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Capability-SSoT-v2

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
- Deterministic manifest entries exist for every changed non-doc file (correct Start/End/LineDelta/PreSHA1/PostSHA1).

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.113.md (recorded_at: 2026-01-18T15:01:56.223Z)
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md 11.1 ([HSK-4001]) and 11.1.3.1-11.1.3.2
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
- Current WP_STATUS:
- What changed in this update:
- Next step / handoff hint:

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
