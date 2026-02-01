# Task Packet: WP-1-Model-Swap-Protocol-v1

## METADATA
- TASK_ID: WP-1-Model-Swap-Protocol-v1
- WP_ID: WP-1-Model-Swap-Protocol-v1
- BASE_WP_ID: WP-1-Model-Swap-Protocol (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-02-01T14:24:12.502Z
- REQUESTOR: ilja (Operator)
- AGENT_ID: user_orchestrator (Codex CLI)
- ROLE: Orchestrator
- CODER_MODEL: <unclaimed>
- CODER_REASONING_STRENGTH: <unclaimed> (LOW | MEDIUM | HIGH | EXTRA_HIGH)
- **Status:** Ready for Dev
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja010220261514
- PACKET_FORMAT_VERSION: 2026-02-01

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-Model-Swap-Protocol-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Implement the ModelSwapRequest protocol (schema + runtime handling) and sequential model swaps, including persisted swap state + resume semantics and required FR-EVT-MODEL telemetry.
- Why: Phase 1 requires deterministic model resource management for multi-model workflows; swaps must be auditable and safe to resume.
- IN_SCOPE_PATHS:
  - docs/task_packets/WP-1-Model-Swap-Protocol-v1.md
  - src/backend/handshake_core/src/mex/runtime.rs
  - src/backend/handshake_core/src/llm/mod.rs
  - src/backend/handshake_core/src/llm/ollama.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
  - src/backend/handshake_core/tests/model_swap_events_tests.rs
  - src/backend/handshake_core/tests/ (new/updated tests for model swap protocol)
- OUT_OF_SCOPE:
  - app/ UI changes (Operator Consoles)
  - Adding new model providers beyond existing runtime(s) unless required to support the swap protocol
  - Non-sequential / concurrent swap execution (spec requires sequential swaps)

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Model-Swap-Protocol-v1
cd src/backend/handshake_core
cargo test
cargo clippy --all-targets --all-features
just cargo-clean
just post-work WP-1-Model-Swap-Protocol-v1
```

### DONE_MEANS
- Backend accepts and validates ModelSwapRequest schema_version "hsk.model_swap@0.4" and rejects invalid/mismatched requests.
- Runtime executes the normative swap sequence (persist -> unload -> load -> ACE recompile -> resume) and supports sequential swaps.
- Persisted swap state includes a deterministic state_hash (sha256 lowercase hex over canonical state bytes) and is verified before resume.
- Flight Recorder emits FR-EVT-MODEL-001..005 with canonical event_id/event_type and rejects unknown variants at ingestion.
- Tests cover: success path, failure path, timeout, and rollback (where applicable), with telemetry assertions.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.123.md (recorded_at: 2026-02-01T14:24:12.502Z)
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.123.md 4.3.3.4.3-4.3.3.4.4 (ModelSwapRequest + Model Swap Protocol) and 11.5.6 (FR-EVT-MODEL-001..005)
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md
- WP Traceability: docs/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- Prior packet (stub): docs/task_packets/stubs/WP-1-Model-Swap-Protocol-v1.md
- Preserved: intent (sequential swaps + persisted state + audit events), anchor candidates, activation checklist.
- Changed: activated into official packet; pinned spec baseline v02.123; added explicit in-scope paths, test plan, and acceptance criteria.

## BOOTSTRAP
- FILES_TO_OPEN:
  - docs/START_HERE.md
  - docs/SPEC_CURRENT.md
  - docs/ARCHITECTURE.md
  - Handshake_Master_Spec_v02.123.md (ModelSwapRequest + Model Swap Protocol + FR-EVT-MODEL)
  - src/backend/handshake_core/src/mex/runtime.rs
  - src/backend/handshake_core/src/llm/mod.rs
  - src/backend/handshake_core/src/llm/ollama.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/workflows.rs
- SEARCH_TERMS:
  - "schema_version: \\\"hsk.model_swap@0.4\\\""
  - "FR-EVT-MODEL-001"
  - "model_swap_requested"
  - "state_hash"
  - "ollama"
- RUN_COMMANDS:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  cargo clippy --manifest-path src/backend/handshake_core/Cargo.toml --all-targets --all-features
  ```
- RISK_MAP:
  - "state_hash mismatch" -> "resume blocked; treat as tamper/mismatch"
  - "swap thrash / VRAM churn" -> "timeouts, instability; enforce budgets and rate limits"
  - "telemetry drift (wrong ids/types)" -> "audit gaps; must enforce canonical event family"

## SKELETON
- Proposed interfaces/types/contracts:
  - ModelSwapRequest parsing/validation (schema_version gate)
  - ModelSwapEngine steps (persist/unload/load/recompile/resume)
  - ModelSwapState persistence + deterministic hashing
- Open questions:
  - Where the swap command is sourced in runtime (capability command vs internal orchestration)
  - Exact persistence location for swap state (storage layer vs runtime-owned file)
- Notes:
  - Ensure swaps are strictly sequential per spec; reject overlapping swaps.

## END_TO_END_CLOSURE_PLAN [CX-E2E-001]
- END_TO_END_CLOSURE_PLAN_APPLICABLE: YES
- TRUST_BOUNDARY: server->model_runtime (ModelSwapRequest -> swap engine -> resumed execution)
- SERVER_SOURCES_OF_TRUTH:
  - persisted ModelSwapState (state_hash recomputed and verified)
  - allowed model ids / capability policy decisions (no client-trusted target_model_id)
- REQUIRED_PROVENANCE_FIELDS:
  - trace_id
  - job_id (when applicable)
  - work_packet_id
  - model_id (before/after)
- VERIFICATION_PLAN:
  - emit FR-EVT-MODEL-* events with correlation ids; validator asserts canonical ids/types
- ERROR_TAXONOMY_PLAN:
  - hash_mismatch (state tamper/mismatch)
  - policy_denied (unauthorized swap)
  - runtime_failure (provider/load/recompile error)
- UI_GUARDRAILS:
  - N/A (backend/runtime)
- VALIDATOR_ASSERTIONS:
  - FR-EVT-MODEL-001..005 canonical ids/types present
  - swap state hash verified before resume
  - sequential swaps only

## IMPLEMENTATION
- (Coder fills after skeleton approval.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Records 'What' hashes/lines for Validator audit. NOT a claim of official Validation.)

### Manifest Entry 1: flight_recorder/mod.rs
- **Target File**: `src/backend/handshake_core/src/flight_recorder/mod.rs`
- **Start**: 1
- **End**: 2810
- **Line Delta**: 231
- **Pre-SHA1**: `287d23e31c1f2971ead4f672610c36cffe8cc70e`
- **Post-SHA1**: `c7f920abf3faa138cfe4db2315487d2c9bb1356e`
- **Change Summary**: Added FR-EVT-MODEL-001..005 event variants and payload validation helpers.
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
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.123.md

### Manifest Entry 2: workflows.rs
- **Target File**: `src/backend/handshake_core/src/workflows.rs`
- **Start**: 1
- **End**: 5660
- **Line Delta**: 772
- **Pre-SHA1**: `6650634199179fdead7b86d80d05fe3284f7110a`
- **Post-SHA1**: `d5f691a9dae7f63de5a0f9ff7e360aed0b27c659`
- **Change Summary**: Implemented ModelSwapRequest v0.4 persistence + state_hash verification, policy extension parsing (exec_policy_ext@0.4), and swap failure/timeout/rollback event emission.
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
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.123.md

### Manifest Entry 3: micro_task_executor_tests.rs
- **Target File**: `src/backend/handshake_core/tests/micro_task_executor_tests.rs`
- **Start**: 1
- **End**: 739
- **Line Delta**: 392
- **Pre-SHA1**: `d72fd69702b3f6810eec62f8f39148bc5e288a3f`
- **Post-SHA1**: `9d70ad49e4bf7116295ea516efaf54dc7716b2eb`
- **Change Summary**: Added model swap telemetry tests for success, policy failure (abort), and timeout with continue_with_current rollback semantics.
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
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.123.md

### Manifest Entry 4: model_swap_events_tests.rs
- **Target File**: `src/backend/handshake_core/tests/model_swap_events_tests.rs`
- **Start**: 1
- **End**: 71
- **Line Delta**: 81
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `1ac07f7a6ed5065a4951945008cf9e562b1dc216`
- **Change Summary**: Added Flight Recorder ingestion validation tests for FR-EVT-MODEL-001..005 canonical types.
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
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.123.md

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS:
- What changed in this update:
- Next step / handoff hint:

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
