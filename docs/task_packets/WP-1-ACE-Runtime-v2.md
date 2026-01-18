# Task Packet: WP-1-ACE-Runtime-v2

## METADATA
- TASK_ID: WP-1-ACE-Runtime-v2
- WP_ID: WP-1-ACE-Runtime-v2
- BASE_WP_ID: WP-1-ACE-Runtime
- DATE: 2026-01-18T16:03:00.013Z
- REQUESTOR: User
- AGENT_ID: CodexCLI-GPT-5.2 (Orchestrator)
- ROLE: Orchestrator
- CODER_MODEL: Claude Opus 4.5 (claude-opus-4-5-20250114)
- CODER_REASONING_STRENGTH: EXTRA_HIGH
- **Status:** In Progress
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja180120261659

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-ACE-Runtime-v2.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Remediate/revalidate ACE-RAG-001 (Retrieval Correctness and Efficiency) against Master Spec v02.113, ensuring QueryPlan/RetrievalTrace schemas, deterministic normalization/hash, required validator trait/guards, and logging requirements match the Main Body text.
- Why: Make retrieval deterministic, auditable, and budget-enforced; eliminate spec drift from the prior WP-1-ACE-Runtime packet and restore Phase 1 closure readiness.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/ace/mod.rs
  - src/backend/handshake_core/src/ace/validators/mod.rs
  - src/backend/handshake_core/src/ace/validators/budget.rs
  - src/backend/handshake_core/src/ace/validators/freshness.rs
  - src/backend/handshake_core/src/ace/validators/drift.rs
  - src/backend/handshake_core/src/ace/validators/cache.rs
  - src/backend/handshake_core/src/ace/validators/determinism.rs
- OUT_OF_SCOPE:
  - Any changes to `src/backend/handshake_core/src/capabilities.rs` or `src/backend/handshake_core/src/workflows.rs` (owned by WP-1-Capability-SSoT-v2)
  - Any changes to storage layer files/migrations owned by WP-1-Mutation-Traceability-v2 (`src/backend/handshake_core/src/storage/**`, `src/backend/handshake_core/src/models.rs`, `src/backend/handshake_core/migrations/`)
  - UI/Operator Consoles surfacing and deep-linking (separate WP)

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- WAIVER-SCOPE-EXPAND-WP-1-ACE-Runtime-v2-001 [CX-573F]
  - Date: 2026-01-18
  - Scope: Expand IN_SCOPE_PATHS beyond this packet as needed to satisfy DONE_MEANS (incl. `src/backend/handshake_core/Cargo.toml` and `Cargo.lock` if dependency changes are required).
  - Justification: Operator explicitly waived out-of-scope gating to unblock implementation.
  - Approver: Operator (chat waiver: "i waive out of scope" / "i waive the scope, it is allowed")
  - Expiry: On WP closure (validation complete).
- WAIVER-NONDETERMINISM-WP-1-ACE-Runtime-v2-002 [CX-573E]
  - Date: 2026-01-18
  - Scope: `src/backend/handshake_core/src/ace/validators/mod.rs:951` - `Instant::now()` call
  - Justification: Timing-only instrumentation for Flight Recorder latency metrics; does not affect validation logic or determinism of ACE guard outcomes.
  - Code marker: `// WAIVER [CX-573E]: timing-only instrumentation for FR latency metrics; no determinism impact`
  - Expiry: On WP closure (validation complete).

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-ACE-Runtime-v2

# Coder (development):
cd src/backend/handshake_core; cargo test
cd src/backend/handshake_core; cargo clippy --all-targets --all-features

# Coder (pre-commit deterministic gate):
just post-work WP-1-ACE-Runtime-v2

# Validator (protocol gates):
just validator-spec-regression
just validator-error-codes
just validator-hygiene-full
```

### DONE_MEANS
- `QueryPlan` and `RetrievalTrace` schemas align to Master Spec v02.113 2.6.6.7.14.5 (fields present and correctly typed/represented in code).
- Deterministic `normalized_query_hash` computed as `sha256(normalize(query_text))` where normalize matches v02.113 2.6.6.7.14.6(B) (trim, collapse whitespace, NFC, Unicode casefold, strip control chars).
- `AceRuntimeValidator` trait aligns to v02.113 2.6.6.7.14.11 and required guard implementations exist (Budget/Freshness/Drift/CacheKey) and are wired into the retrieval flow.
- Logging requirements for retrieval-backed calls are satisfied per v02.113 2.6.6.7.14.12 (QueryPlan id+hash, normalized_query_hash, RetrievalTrace id+hash, cache hits/misses, rerank + diversity metadata).
- Conformance tests for ACE-RAG-001 are implemented/updated and passing where applicable per v02.113 2.6.6.7.14.13 (at minimum: T-ACE-RAG-001 and T-ACE-RAG-002; expand toward T-ACE-RAG-003..007 as feasible within scope).

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.113.md (recorded_at: 2026-01-18T16:03:00.013Z)
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md 2.6.6.7.14.5, 2.6.6.7.14.6(B), 2.6.6.7.14.11, 2.6.6.7.14.12, 2.6.6.7.14.13 (ACE-RAG-001)
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md
- WP Traceability: docs/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- Prior packet: docs/task_packets/WP-1-ACE-Runtime.md (historical; spec drift revalidation FAIL).
- Stub: docs/task_packets/stubs/WP-1-ACE-Runtime-v2.md (planning stub; not executable).
- Preserved scope: ACE-RAG-001 QueryPlan/RetrievalTrace + deterministic normalization/hash + validator trait/guards + conformance tests.
- Updated in v2: re-anchored to Master Spec v02.113 and refreshed acceptance criteria and evidence expectations for validators/logging/conformance.

## BOOTSTRAP
- FILES_TO_OPEN:
  - docs/START_HERE.md
  - docs/SPEC_CURRENT.md
  - docs/ARCHITECTURE.md
  - Handshake_Master_Spec_v02.113.md
  - docs/task_packets/WP-1-ACE-Runtime.md
  - src/backend/handshake_core/src/ace/mod.rs
  - src/backend/handshake_core/src/ace/validators/mod.rs
  - src/backend/handshake_core/src/ace/validators/budget.rs
  - src/backend/handshake_core/src/ace/validators/freshness.rs
  - src/backend/handshake_core/src/ace/validators/drift.rs
  - src/backend/handshake_core/src/ace/validators/cache.rs
  - src/backend/handshake_core/src/ace/validators/determinism.rs
- SEARCH_TERMS:
  - "ACE-RAG-001"
  - "QueryPlan"
  - "RetrievalTrace"
  - "normalized_query_hash"
  - "normalize("
  - "AceRuntimeValidator"
  - "RetrievalBudgetGuard"
  - "ContextPackFreshnessGuard"
  - "IndexDriftGuard"
  - "CacheKeyGuard"
- RUN_COMMANDS:
  ```bash
  cd src/backend/handshake_core; cargo test
  ```
- RISK_MAP:
  - "non-deterministic normalization/hash" -> "cache misses, audit drift, replay breakage"
  - "guards not applied universally" -> "hidden retrieval bypasses budgets/freshness/drift"
  - "logging incomplete" -> "hollow audit; Flight Recorder cannot reconstruct retrieval decisions"

## SKELETON

### normalize_query() Spec Alignment
- **Location**: `ace/mod.rs:437-487`
- **Spec**: v02.113 2.6.6.7.14.6(B)
- **Implementation**:
  1. NFC normalize via `unicode_normalization::UnicodeNormalization::nfc()`
  2. Convert whitespace (including \t \n \r) to ASCII space
  3. Strip non-whitespace control chars (NUL, BEL, BS, etc.)
  4. Apply Unicode casefold via `caseless::default_case_fold_str()`
  5. Collapse whitespace runs to single space
  6. Trim leading/trailing whitespace

### Flight Recorder Logging Integration
- **Location**: `ace/validators/mod.rs:709-989`
- **Spec**: v02.113 2.6.6.7.14.12
- **New Types**:
  - `AceValidationPayload`: Full ace_validation sub-object for llm_inference events
  - `CacheMarker`: Per-stage cache hit/miss tracking
- **New Method**: `ValidatorPipeline::validate_and_log()` returns (errors, payload)
- **Payload Fields**: scope_document_id, determinism_mode, candidate_ids/hashes, selected_ids/hashes, query_plan_id/hash, normalized_query_hash, retrieval_trace_id/hash, rerank/diversity metadata, cache_markers, drift_flags, guards_passed/failed, violation_codes, outcome

### T-ACE-RAG-003 Definition
- **Location**: `ace/mod.rs:1173-1324`
- **Spec**: v02.113 2.6.6.7.14.13
- **Test**: `test_replay_persistence_correctness`
- **Proves**: Under replay mode, serialized then deserialized RetrievalTrace produces identical candidate_ids, selected_ids, rerank hashes, and full trace hash

### Open Questions
- (Resolved) Casefold crate: Using `caseless` v0.2

### Notes
- Waiver WAIVER-SCOPE-EXPAND-WP-1-ACE-Runtime-v2-001 covers Cargo.toml/Cargo.lock changes

## IMPLEMENTATION
- (Coder fills after skeleton approval.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Records 'What' hashes/lines for Validator audit. NOT a claim of official Validation.)

### Manifest Entry 1: Cargo.toml
- **Target File**: `src/backend/handshake_core/Cargo.toml`
- **Start**: 26
- **End**: 27
- **Line Delta**: 1
- **Pre-SHA1**: `e437bd6391dc446bf9e578e23bc55394382778ec`
- **Post-SHA1**: `114459e671ec0f838ed0545dbf89b89949c32b58`
- **Change Summary**: Added caseless 0.2 dependency for Unicode casefold
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

### Manifest Entry 2: Cargo.lock
- **Target File**: `src/backend/handshake_core/Cargo.lock`
- **Start**: 1
- **End**: 10
- **Line Delta**: 10
- **Pre-SHA1**: `5593f6381e5a819fd9dc599780be0e9a52ffff7a`
- **Post-SHA1**: `5b05c07f94170c1f98a7ee696c4e6552638628f9`
- **Change Summary**: Lock file updated with caseless v0.2.2 dependency
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

### Manifest Entry 3: ace/mod.rs
- **Target File**: `src/backend/handshake_core/src/ace/mod.rs`
- **Start**: 1
- **End**: 1322
- **Line Delta**: 204
- **Pre-SHA1**: `dbaa52678d143cd718fbbcaf84e7a80428d0545f`
- **Post-SHA1**: `917e5c75a06f8246ea0425b1de2d8fbaa1d99458`
- **Change Summary**: Updated spec ref v02.85 to v02.113; Fixed normalize_query for casefold and strip; Added T-ACE-RAG-001b casefold test; Added T-ACE-RAG-003 replay persistence test; Removed forbidden patterns (expect/unwrap); Updated re-exports
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

### Manifest Entry 4: ace/validators/mod.rs
- **Target File**: `src/backend/handshake_core/src/ace/validators/mod.rs`
- **Start**: 1
- **End**: 1265
- **Line Delta**: 306
- **Pre-SHA1**: `8d265514d658595afede656d72d11fbb3b87f89f`
- **Post-SHA1**: `727f75c0d3eb801ae0e081f8b3d1a7d4f0b26590`
- **Change Summary**: Added AceValidationPayload struct for FR logging; Added CacheMarker struct; Added ValidatorPipeline validate_and_log method; Added WAIVER [CX-573E] for Instant::now()
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

### Manifest Entry 5: flight_recorder/mod.rs (cherry-pick gate-unblock)
- **Target File**: `src/backend/handshake_core/src/flight_recorder/mod.rs`
- **Start**: 1
- **End**: 1501
- **Line Delta**: 11
- **Pre-SHA1**: `ee10486cbd46eac5ee903dbfc9adf43afb07ee6b`
- **Post-SHA1**: `6cc6375588b4cad93103fca66324a4c97e284060`
- **Change Summary**: Cherry-pick Coder2 gate-unblock commits (9190e673, f8237f90) - removed expect() from ExportRecord serialization
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

- **Lint Results**: cargo clippy passed (warnings present)
- **Artifacts**: Cargo.lock updated with caseless v0.2.2
- **Timestamp**: 2026-01-18
- **Spec Target Resolved**: docs/SPEC_CURRENT.md to Handshake_Master_Spec_v02.113.md
- **Notes**: Waiver WAIVER-SCOPE-EXPAND-WP-1-ACE-Runtime-v2-001 covers Cargo.toml, Cargo.lock, and flight_recorder/mod.rs changes

## STATUS_HANDOFF
- Current WP_STATUS: Implementation Complete - Ready for Validation
- What changed in this update:
  1. **Cargo.toml**: Added `caseless = "0.2"` for Unicode casefold support
  2. **ace/mod.rs**:
     - Updated spec reference v02.85 -> v02.113 (line 9)
     - Fixed normalize_query() to use true Unicode casefold (caseless::default_case_fold_str) and strip non-whitespace control chars (lines 437-487)
     - Added test_unicode_casefold_correctness test proving casefold (lines 974-1015)
     - Added test_replay_persistence_correctness (T-ACE-RAG-003) test (lines 1171-1321)
     - Removed forbidden patterns: replaced expect()/unwrap() with `?` operator and infallible Uuid::from_u128()
     - Updated re-exports for AceValidationPayload, CacheMarker (lines 893-922)
  3. **ace/validators/mod.rs**:
     - Added AceValidationPayload struct for FR logging (lines 709-763)
     - Added CacheMarker struct (lines 766-771)
     - Added AceValidationPayload::from_plan_and_trace() builder (lines 773-917)
     - Added ValidatorPipeline::validate_and_log() method (lines 925-985)
     - Added WAIVER [CX-573E] for Instant::now() timing instrumentation (line 950)
- Next step / handoff hint: Validator to run `just validator-spec-regression`, `just validator-error-codes`, `just validator-hygiene-full` and verify DONE_MEANS criteria

## EVIDENCE

### E1: pre-work validation (2026-01-18)
```
$ just pre-work WP-1-ACE-Runtime-v2
Checking Phase Gate for WP-1-ACE-Runtime-v2...
? GATE PASS: Workflow sequence verified.

Pre-work validation for WP-1-ACE-Runtime-v2...

Check 1: Task packet file exists
PASS: Found WP-1-ACE-Runtime-v2.md

Check 2: Task packet structure
PASS: All required fields present

Check 2.7: Technical Refinement gate
PASS: Refinement file exists and is approved/signed

Check 2.8: WP checkpoint commit gate

Check 3: Deterministic manifest template
PASS: Manifest fields present
PASS: Gates checklist present

==================================================
Pre-work validation PASSED
```

### E2: T-ACE-RAG-001b Unicode casefold test (2026-01-18)
```
$ cargo test ace::tests::test_unicode_casefold_correctness -- --nocapture
running 1 test
test ace::tests::test_unicode_casefold_correctness ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 141 filtered out
```

### E3: T-ACE-RAG-003 Replay persistence test (2026-01-18)
```
$ cargo test ace::tests::test_replay_persistence_correctness -- --nocapture
running 1 test
test ace::tests::test_replay_persistence_correctness ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 141 filtered out
```

### E4: All ACE tests passing (2026-01-18)
```
$ cargo test ace::
running 81 tests
test ace::tests::test_budget_validation ... ok
test ace::tests::test_context_pack_staleness ... ok
test ace::tests::test_strict_ranking_determinism ... ok
test ace::tests::test_unicode_casefold_correctness ... ok
test ace::tests::test_query_normalization_determinism ... ok
test ace::tests::test_retrieval_trace_metrics ... ok
test ace::tests::test_cache_key_hashing ... ok
test ace::tests::test_replay_persistence_correctness ... ok
[...all validator tests pass...]

test result: ok. 81 passed; 0 failed; 0 ignored; 0 measured; 61 filtered out
```

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
