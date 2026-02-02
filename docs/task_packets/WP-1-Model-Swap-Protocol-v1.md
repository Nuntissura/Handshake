# Task Packet: WP-1-Model-Swap-Protocol-v1

## METADATA
- TASK_ID: WP-1-Model-Swap-Protocol-v1
- WP_ID: WP-1-Model-Swap-Protocol-v1
- BASE_WP_ID: WP-1-Model-Swap-Protocol (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-02-01T14:24:12.502Z
- REQUESTOR: ilja (Operator)
- AGENT_ID: user_orchestrator (Codex CLI)
- ROLE: Orchestrator
- CODER_MODEL: GPT-5.2 (Codex CLI)
- CODER_REASONING_STRENGTH: EXTRA_HIGH
- **Status:** In Progress
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
just post-work WP-1-Model-Swap-Protocol-v1 --range 5e3781b3..HEAD
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
  - `LlmClient::swap_model(req: ModelSwapRequestV0_4) -> Result<(), LlmError>` (provider adapter primitive)
  - `ModelSwapRequestV0_4` validation + persistence (request + state) with deterministic `state_hash`
  - MT executor swap sequence (persist -> emit -> unload/load -> compile -> resume -> completion/failure)
- Open questions:
  - (Deferred) centralized model catalog/budgeting beyond the Phase-1 hardcoded allowlist
  - (Deferred) explicit provider unload API beyond best-effort keepalive controls
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
- Implemented a real runtime swap primitive on the LLM abstraction: `LlmClient::swap_model` with a default unsupported implementation (other adapters compile cleanly).
- Implemented `swap_model` for Ollama using best-effort unload/load via `/api/generate` with `keep_alive` plus `tokio::time::timeout` fail-fast behavior.
- Updated MT executor model escalation path to execute the normative swap sequence:
  - Persist swap state (including state_hash derived from referenced artifact bytes)
  - Emit FR-EVT-MODEL-001 (requested)
  - Enforce budgets and timeout
  - Execute runtime swap (`swap_model`) and handle failure/timeout/rollback per fallback strategy
  - Emit completion only after a fresh post-swap context compile artifact exists (FR-EVT-MODEL-002).
- Fixed `state_hash` semantics to hash persisted artifact contents (not swap_state JSON bytes) and removed request/state files from `state_persist_refs` to avoid circular hashing.
- Updated MT executor tests to assert:
  - runtime swap is invoked
  - state_hash recomputes from refs deterministically
  - completion is emitted only after `context_compile_ref` exists
  - failure/timeout/rollback telemetry paths are exercised.

## HYGIENE
- Ran `just pre-work WP-1-Model-Swap-Protocol-v1` (see evidence log + sha256).
- Ran `cargo test` (see evidence log + sha256).
- Ran `cargo clippy --all-targets --all-features` (see evidence log + sha256).
- Ran `just cargo-clean` (see evidence log + sha256).
- Ran `just post-work WP-1-Model-Swap-Protocol-v1 --range 5e3781b3..HEAD` (see evidence log + sha256).

## VALIDATION
- (Mechanical manifest for audit. Records 'What' hashes/lines for Validator audit. NOT a claim of official Validation.)

### Manifest Entry 1: flight_recorder/mod.rs
- **Target File**: `src/backend/handshake_core/src/flight_recorder/mod.rs`
- **Start**: 1
- **End**: 3058
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

### Manifest Entry 2: llm/mod.rs
- **Target File**: `src/backend/handshake_core/src/llm/mod.rs`
- **Start**: 1
- **End**: 287
- **Line Delta**: 13
- **Pre-SHA1**: `fffdbcd7036a266050801d9e8113f07cccf92c77`
- **Post-SHA1**: `a72c324cc956fa6647ac90651e7f2696a33c6327`
- **Change Summary**: Added `LlmClient::swap_model` as a runtime primitive (default unsupported) for model swaps.
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

### Manifest Entry 3: llm/ollama.rs
- **Target File**: `src/backend/handshake_core/src/llm/ollama.rs`
- **Start**: 1
- **End**: 812
- **Line Delta**: 83
- **Pre-SHA1**: `9013b0396e3621175ac96a6fc1b82e54f0ee4333`
- **Post-SHA1**: `98204f70da0b822a9cdc7d16c364537d63a0898b`
- **Change Summary**: Implemented best-effort unload/load via `keep_alive` and added a `swap_model` adapter method with timeout behavior.
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

### Manifest Entry 4: workflows.rs
- **Target File**: `src/backend/handshake_core/src/workflows.rs`
- **Start**: 1
- **End**: 6582
- **Line Delta**: 1117
- **Pre-SHA1**: `6650634199179fdead7b86d80d05fe3284f7110a`
- **Post-SHA1**: `adc310811af0e9a86ad9f723aa879324ed005016`
- **Change Summary**: Implemented MT executor runtime swap engine (persist -> emit -> swap -> compile -> resume), state_hash over persisted artifacts, budget+timeout enforcement, and completion gating after post-swap context compile artifact exists.
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

### Manifest Entry 5: micro_task_executor_tests.rs
- **Target File**: `src/backend/handshake_core/tests/micro_task_executor_tests.rs`
- **Start**: 1
- **End**: 1117
- **Line Delta**: 670
- **Pre-SHA1**: `d72fd69702b3f6810eec62f8f39148bc5e288a3f`
- **Post-SHA1**: `c98ec5ca6fc6bf60162e5d5f2aba118d2d5a853d`
- **Change Summary**: Added swap runtime assertions (swap invoked, compile artifact exists before completion), state_hash recomputation from refs, and failure/timeout/rollback coverage.
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

### Manifest Entry 6: model_swap_events_tests.rs
- **Target File**: `src/backend/handshake_core/tests/model_swap_events_tests.rs`
- **Start**: 1
- **End**: 81
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
- Current WP_STATUS: Implementation complete on branch; ready for Validator review.
- What changed in this update:
  - Commit `ce033773`: Implemented runtime model swap primitive + MT executor swap engine + tests.
  - Commit `3e819d6a`: Updated this task packet (implementation/hygiene/manifest/evidence).
- Next step / handoff hint:
  - Commit packet updates, then request Validator review.

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Evidence logs (paths + sha256):
  - `data/wp_evidence/WP-1-Model-Swap-Protocol-v1/just_pre_work.txt` sha256=BE5F71E49FE59149A2585424BB19EFD01AFEFD1D611CFF6797B538303EDE17C3
  - `data/wp_evidence/WP-1-Model-Swap-Protocol-v1/cargo_test.txt` sha256=08F210614B41E0EE33831CA832C614097A4274C645FE887ABF0725A548A4976C
  - `data/wp_evidence/WP-1-Model-Swap-Protocol-v1/cargo_clippy.txt` sha256=D9E9557628C13C5F409E44CA479459E6F1EF3E6602B72CA194F8170CF08EBCED
  - `data/wp_evidence/WP-1-Model-Swap-Protocol-v1/just_cargo_clean.txt` sha256=28A9F9C94369CDF3EC0F1C50B8A06182A21559C7C0093EA509E91DDF31D71677
  - `data/wp_evidence/WP-1-Model-Swap-Protocol-v1/just_post_work_range.txt` sha256=DC6F507CB5EEB4D14A5D72453F9AD65D48FA7A4673B335D4A70162B6540F182B
  - `data/wp_evidence/WP-1-Model-Swap-Protocol-v1/just_post_work_range_latest.txt` sha256=7254660124079B7E528CDA574565976620E12C82248896F106E5DD2115D1570B

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)

### VALIDATION REPORT — WP-1-Model-Swap-Protocol-v1
- Date: 2026-02-02
- Validator: ROLE=VALIDATOR (external revalidation)
- Verdict: PASS

#### REASON FOR PASS
- All normative requirements referenced in the packet (Master Spec v02.123 anchors §4.3.3.4.3–§4.3.3.4.4 and §11.5.6) have concrete implementation evidence and behavioral test coverage.
- Swap completion is gated on a fresh post-swap context snapshot + compile artifact (prevents “telemetry-only completion”).
- Hygiene gates and packet TEST_PLAN commands were executed; no forbidden patterns detected in backend sources.

#### Scope Inputs
- Task Packet: `docs/task_packets/WP-1-Model-Swap-Protocol-v1.md` (Status: In Progress; RISK_TIER: HIGH; USER_SIGNATURE: ilja010220261514)
- Binding Spec: `docs/SPEC_CURRENT.md` → `Handshake_Master_Spec_v02.123.md`
  - §4.3.3.4.3 ModelSwapRequest (Normative)
  - §4.3.3.4.4 Model Swap Protocol (Normative)
  - §11.5.6 FR-EVT-MODEL-001..005 (Normative)

#### Evidence Mapping (Spec → Code)
- §4.3.3.4.3 ModelSwapRequest (typed shape + validation)
  - Schema/enums/struct: `src/backend/handshake_core/src/workflows.rs:190`
  - Validation (schema_version, bounded ids/refs, sha256 lowercase hex): `src/backend/handshake_core/src/workflows.rs:374`
- §4.3.3.4.4 Protocol steps 1–2 (persist state + emit requested)
  - Persist helpers + state_hash derivation: `src/backend/handshake_core/src/workflows.rs:490`
  - Persist + verify before swap continues: `src/backend/handshake_core/src/workflows.rs:5510`
  - Emit requested: `src/backend/handshake_core/src/workflows.rs:5520`
- §4.3.3.4.4 Protocol steps 3–4 (unload/offload + load target) + timeout enforcement
  - Runtime primitive surface: `src/backend/handshake_core/src/llm/mod.rs:27`
  - Provider implementation (Ollama): `src/backend/handshake_core/src/llm/ollama.rs:444`
  - Workflow enforces timeout: `src/backend/handshake_core/src/workflows.rs:5763`
- §4.3.3.4.4 Protocol steps 5–7 (fresh context compile + resume + completion/failure/rollback events)
  - Completion emitted only after fresh context snapshot and compile artifact is written: `src/backend/handshake_core/src/workflows.rs:4849`
  - Resume on new model level: `src/backend/handshake_core/src/workflows.rs:5915`
- §11.5.6 FR-EVT-MODEL-001..005 ingestion validation
  - Event variants: `src/backend/handshake_core/src/flight_recorder/mod.rs:80`
  - Payload validator: `src/backend/handshake_core/src/flight_recorder/mod.rs:1578`

#### Tests (Behavioral)
- Model swap is not telemetry-only; runtime primitive must be invoked:
  - `src/backend/handshake_core/tests/micro_task_executor_tests.rs:75`
  - `src/backend/handshake_core/tests/micro_task_executor_tests.rs:459`
- state_hash is recomputed from referenced artifacts and must match:
  - `src/backend/handshake_core/tests/micro_task_executor_tests.rs:544`
- Completion gating: compile artifact exists before ModelSwapCompleted is emitted:
  - `src/backend/handshake_core/tests/micro_task_executor_tests.rs:564`
- Timeout → rollback path:
  - `src/backend/handshake_core/tests/micro_task_executor_tests.rs:868`
- Flight Recorder ingestion validation: accept/reject payload shape:
  - `src/backend/handshake_core/tests/model_swap_events_tests.rs:1`

#### Validation Commands Executed
- `just pre-work WP-1-Model-Swap-Protocol-v1`
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`
- `cargo clippy --manifest-path src/backend/handshake_core/Cargo.toml --all-targets --all-features`
- `just cargo-clean`
- `just post-work WP-1-Model-Swap-Protocol-v1 --range 5e3781b3..HEAD`
- `just validator-spec-regression`
- `just validator-error-codes`
- `just validator-dal-audit`
- `just validator-scan`

#### Non-Blocking Notes
- `record_model_swap_event_v0_4` currently emits `"role": "worker"` rather than `request.role`; MT executor path is worker-only today, but other subsystems may need true role emission later (`src/backend/handshake_core/src/workflows.rs:309`).

Verdict: PASS
