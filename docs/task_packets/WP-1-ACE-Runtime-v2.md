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
- What: Remediate/revalidate ACE-RAG-001 (Retrieval Correctness \u0026 Efficiency) against Master Spec v02.113, ensuring QueryPlan/RetrievalTrace schemas, deterministic normalization/hash, required validator trait/guards, and logging requirements match the Main Body text.
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
- NONE

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
- Proposed interfaces/types/contracts:
- Open questions:
- Notes:

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
