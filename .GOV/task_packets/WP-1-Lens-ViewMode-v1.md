# Task Packet: WP-1-Lens-ViewMode-v1

## METADATA
- TASK_ID: WP-1-Lens-ViewMode-v1
- WP_ID: WP-1-Lens-ViewMode-v1
- BASE_WP_ID: WP-1-Lens-ViewMode (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-02-24T12:03:19.298Z
- MERGE_BASE_SHA: 13efad9235ec5e7cfe4870bcb986b81d30416aed
- REQUESTOR: Operator (ilja)
- AGENT_ID: CodexCLI-GPT-5.2 (Orchestrator)
- ROLE: Orchestrator
- AGENTIC_MODE: YES
- ORCHESTRATOR_MODEL: GPT-5.2 (Codex CLI) (required if AGENTIC_MODE=YES)
- ORCHESTRATION_STARTED_AT_UTC: 2026-02-24T14:23:12.007Z
- CODER_MODEL: GPT-5.2 (Codex CLI)
- CODER_REASONING_STRENGTH: HIGH
- **Status:** In Progress
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja240220261300
- PACKET_FORMAT_VERSION: 2026-02-01

## SUB_AGENT_DELEGATION (OPTIONAL; OPERATOR-GATED)
- SUB_AGENT_DELEGATION: DISALLOWED
- OPERATOR_APPROVAL_EVIDENCE: N/A
- SUB_AGENT_REASONING_ASSUMPTION: LOW (HARD)
- RULES (if SUB_AGENT_DELEGATION=ALLOWED):
  - Sub-agents produce draft code only; Primary Coder verifies against SPEC_CURRENT + task packet acceptance criteria before applying.
  - Sub-agents MUST NOT edit any governance surface (`.GOV/**`, including task packets/refinements and `## VALIDATION_REPORTS`).
  - Only Primary Coder runs gates, records EVIDENCE/EVIDENCE_MAPPING/VALIDATION manifest, commits, and hands off.
  - See: `/.GOV/roles/coder/agentic/AGENTIC_PROTOCOL.md` Section 6.
- NOTE: Set `SUB_AGENT_DELEGATION: ALLOWED` only with explicit Operator approval; when ALLOWED, replace `OPERATOR_APPROVAL_EVIDENCE` with the exact approval line from chat.

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/WP-1-Lens-ViewMode-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Implement ViewMode (SFW/NSFW) as an operator-controlled projection filter for Lens retrieval + output, including strict hard-drop behavior in SFW.
- Why: Enforce content-governance posture without mutating stored artifacts and without "collapsed but revealable" leakage in SFW mode.
- IN_SCOPE_PATHS:
  - .GOV/refinements/WP-1-Lens-ViewMode-v1.md
  - .GOV/task_packets/WP-1-Lens-ViewMode-v1.md
  - .GOV/roles_shared/TASK_BOARD.md
  - .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md
  - src/backend/handshake_core/src/ace/mod.rs
  - src/backend/handshake_core/src/ace/validators/mod.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/tests/lens_viewmode_tests.rs
  - app/src/App.tsx
  - app/src/App.css
  - app/src/lib/api.ts
  - app/src/lib/viewMode.ts
  - app/src/components/ViewModeToggle.tsx
- OUT_OF_SCOPE:
  - Any write-back or mutation of stored descriptors/facts/artifacts to achieve SFW (projection-only).
  - Any "collapsed/blurred but revealable" UI for non-sfw items when ViewMode="SFW" (hard drop only).
  - Unified Tool Surface Contract WP file locks (do not touch):
    - assets/schemas/htc_v1.json
    - src/backend/handshake_core/src/mcp/gate.rs
    - src/backend/handshake_core/src/mcp/fr_events.rs
    - src/backend/handshake_core/src/mex/runtime.rs
    - src/backend/handshake_core/src/mex/conformance.rs
    - src/backend/handshake_core/src/flight_recorder/mod.rs
    - src/backend/handshake_core/src/flight_recorder/duckdb.rs
    - src/backend/handshake_core/tests/mcp_gate_tests.rs

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Lens-ViewMode-v1

# Governance + formatting + lint:
just gov-check
just fmt
just lint

# Tests:
cd app; pnpm test
just test

# Post-work (before PR / final handoff):
just post-work WP-1-Lens-ViewMode-v1 --range 13efad9235ec5e7cfe4870bcb986b81d30416aed..HEAD
```

### DONE_MEANS
- ViewMode type exists per spec and is plumbed through Lens query inputs as \"NSFW\"|\"SFW\" (default \"NSFW\").
- In ViewMode=\"SFW\", retrieval enforces strict drop (default-deny): no returned candidates/results with content_tier != \"sfw\"; content_tier==None is treated as non-sfw and dropped (no \"collapsed/blurred but revealable\" leakage).
- SFW mode applies projection at render/output only and MUST NOT write back or mutate stored raw/derived descriptors/artifacts.
- In ViewMode=\"SFW\", any SFW-projected output is labeled per spec Addendum 11.3 (required): projection_applied=true, projection_kind=\"SFW\", projection_ruleset_id, and a link to underlying raw evidence.
- QueryPlan and RetrievalTrace record ViewMode as a metadata filter (e.g., QueryPlan.filters.view_mode and/or trace fields), and serialized trace evidence includes it.
- Tests cover SFW hard-drop + trace recording (Rust unit tests; optional frontend test for toggle state/param).

### ROLLBACK_HINT
```bash
# Prefer revert (safe on shared branches):
git revert COMMIT_SHA
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.137.md (recorded_at: 2026-02-24T12:03:19.298Z)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR:
  - Handshake_Master_Spec_v02.137.md Addendum 2.4 ViewMode (SFW/NSFW)
  - Handshake_Master_Spec_v02.137.md 6.3.3.5.7.22 NSFW/SFW policy (raw ingest; filtered view/output only)
  - Handshake_Master_Spec_v02.137.md 2.3.14.x Hybrid Search + Two-Stage Retrieval (Lens filters must be recorded in trace)
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- Prior packets:
  - `.GOV/task_packets/stubs/WP-1-Lens-ViewMode-v1.md` (stub; non-executable)
- Preserved requirements:
  - ViewMode UI + enforcement for Lens outputs (default NSFW; explicit SFW toggle).
  - SFW hard-drop: strict exclusion of non-sfw candidates/results.
  - Projection-only: never mutate stored descriptors/artifacts as part of ViewMode.
  - Traceability: record ViewMode as a metadata filter in QueryPlan/RetrievalTrace.
- Changes in v1 packet:
  - Activated the stub into an official executable packet (`.GOV/task_packets/`) with signed Technical Refinement + recorded PREPARE.
  - Added explicit concurrency/file-lock constraint to avoid overlapping Unified Tool Surface Contract WP files.

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - .GOV/roles_shared/ARCHITECTURE.md
  - Handshake_Master_Spec_v02.137.md (Addendum 2.4; 6.3.3.5.7.22; 2.3.14.x)
  - .GOV/refinements/WP-1-Lens-ViewMode-v1.md
  - .GOV/task_packets/stubs/WP-1-Lens-ViewMode-v1.md
  - src/backend/handshake_core/src/ace/mod.rs
  - src/backend/handshake_core/src/ace/validators/mod.rs
  - src/backend/handshake_core/src/workflows.rs
  - app/src/App.tsx
  - app/src/lib/api.ts
- SEARCH_TERMS:
  - "ViewMode"
  - "\"SFW\""
  - "\"NSFW\""
  - "content_tier"
  - "content_tier_allowlist"
  - "RetrievalFilters"
  - "QueryPlan"
  - "RetrievalTrace"
  - "projection"
  - "strict drop"
- RUN_COMMANDS:
  ```bash
  rg -n "ViewMode|\\bSFW\\b|\\bNSFW\\b|content_tier|content_tier_allowlist|RetrievalFilters|QueryPlan|RetrievalTrace" src/backend/handshake_core/src app/src
  just fmt
  just lint
  cd app; pnpm test
  just test
  ```
- RISK_MAP:
  - "SFW leakage" -> "non-sfw content appears in SFW mode; governance incident"
  - "Write-back mutation" -> "stored artifacts corrupted by projection logic"
  - "Surface inconsistency" -> "some result views ignore ViewMode; bypass"
  - "Trace omission" -> "cannot audit which ViewMode was used for a result set"

## SKELETON
- Proposed interfaces/types/contracts:
  - Rust (`src/backend/handshake_core/src/ace/mod.rs`):
    - `enum ViewMode { "NSFW", "SFW" }` (default: `"NSFW"`; serde SCREAMING_SNAKE_CASE)
    - `enum ContentTier { "sfw", "adult_soft", "adult_explicit" }` (serde snake_case)
    - `enum ProjectionKind { "SFW" }` (serde SCREAMING_SNAKE_CASE)
    - Extend `RetrievalFilters` with `view_mode: ViewMode`
    - (Optional) Type `content_tier_allowlist` as `Option<Vec<ContentTier>>` instead of `Option<Vec<String>>`
    - Extend `RetrievalCandidate` with `content_tier: Option<ContentTier>` (`None` means unknown/unclassified)
    - Extend `RetrievalTrace` with `filters_applied: RetrievalFilters` (serialized trace includes `view_mode`)
    - Extend `RetrievalTrace` with projection labeling fields (spec Addendum 11.3):
      - `projection_applied: bool`
      - `projection_kind: Option<ProjectionKind>`
      - `projection_ruleset_id: Option<String>`
      - Evidence link is preserved via `SelectedEvidence.candidate_ref` and `SpanExtraction.source_ref` (projection never destroys the evidence pointer for remaining items).
    - Add `RetrievalTrace::apply_view_mode_hard_drop()`:
      - if `view_mode=="SFW"`: remove any `candidates`/`selected`/`spans` where `content_tier != sfw` (including `None`)
      - if `view_mode=="SFW"`: set `projection_applied=true`, `projection_kind=Some("SFW")`, and `projection_ruleset_id="viewmode_sfw_hard_drop@v1"`
      - record a warning when drops occur (for audit) without leaking dropped content
  - Rust (`src/backend/handshake_core/src/ace/validators/mod.rs`):
    - Add `ViewModeHardDropGuard` to `ValidatorPipeline::with_default_guards()`:
      - if `plan.filters.view_mode=="SFW"`: assert trace contains no non-sfw or unknown-tier items; else `AceError::ValidationFailed`
  - Rust (`src/backend/handshake_core/src/workflows.rs`):
    - No behavior change for existing flows (default `view_mode=="NSFW"`).
    - Future Lens retrieval path calls `trace.apply_view_mode_hard_drop()` before returning results and before validation logging.
  - Tests (`src/backend/handshake_core/tests/lens_viewmode_tests.rs`):
    - SFW hard-drop filtering (candidates/selected/spans)
    - Trace serialization includes `view_mode`
    - In SFW mode, trace serialization includes projection labeling fields and retains evidence links (SourceRef/candidate_ref).
- Open questions:
  - Source of `content_tier` for real Lens candidates is not yet implemented in current Phase 1 storage models (blocks expose `sensitivity`/`exportable`, not `content_tier`).
- Notes:
  - END_TO_END_CLOSURE_PLAN is already filled in the section below (trust boundary + provenance + error taxonomy).
  - Decision: In `ViewMode=="SFW"`, `content_tier==None` is treated as non-sfw and dropped (strict drop is default-deny).
  - Frontend work in this WP is a global toggle + persistence + labeling; Lens query plumbing will be wired when a Lens query API exists.

## END_TO_END_CLOSURE_PLAN [CX-E2E-001]
- END_TO_END_CLOSURE_PLAN_APPLICABLE: YES
- TRUST_BOUNDARY: client->server (ViewMode toggle influences retrieval + output projection; server must enforce)
- SERVER_SOURCES_OF_TRUTH:
  - Stored artifact/descriptor governance fields (content_tier) are loaded from storage and MUST NOT be client-supplied.
  - ViewMode is validated server-side (enum allowlist); invalid values are rejected.
- REQUIRED_PROVENANCE_FIELDS:
  - view_mode (\"NSFW\"|\"SFW\")
  - content_tier (per returned item / candidate, when applicable)
  - projection_applied + projection_kind + projection_ruleset_id (when view_mode=\"SFW\")
  - evidence link to underlying raw item (e.g., `SelectedEvidence.candidate_ref` / `SpanExtraction.source_ref`)
  - trace_id + query_plan_id (link results to QueryPlan/RetrievalTrace)
- VERIFICATION_PLAN:
  - Add unit tests that prove strict drop behavior in SFW and that QueryPlan/Trace serialization contains view_mode.
  - Add a regression test that ensures SFW projection code paths do not write back/mutate stored artifacts.
- ERROR_TAXONOMY_PLAN:
  - invalid_view_mode_value (client sent bad enum)
  - missing_or_unknown_content_tier (server cannot safely classify; MUST default-deny in SFW)
  - trace_record_missing_view_mode (auditability regression)
- UI_GUARDRAILS:
  - Default ViewMode=\"NSFW\"; SFW requires explicit operator toggle and shows an always-visible label.
  - No \"collapsed/blurred but revealable\" affordance for non-sfw items when SFW.
- VALIDATOR_ASSERTIONS:
  - ViewMode is enforced server-side (cannot be bypassed by client/UI changes).
  - ViewMode is recorded in trace (QueryPlan/RetrievalTrace) and is included in evidence.
  - SFW projection labeling fields exist (Addendum 11.3) and preserve evidence links.
  - SFW is projection-only (no storage mutation) and strict drop is proven via tests.

## IMPLEMENTATION
- Backend (ACE):
  - Added `ViewMode`, `ContentTier`, `ProjectionKind` + serde mappings.
  - Extended `RetrievalFilters` with `view_mode` and `RetrievalCandidate` with `content_tier: Option<ContentTier>`.
  - Extended `RetrievalTrace` with `filters_applied` plus projection labeling fields (`projection_applied`, `projection_kind`, `projection_ruleset_id`).
  - Implemented `RetrievalTrace::apply_view_mode_hard_drop()` (SFW strict-drop + count-only warnings + required labeling).
  - Added `ViewModeHardDropGuard` and wired into default `ValidatorPipeline`.
  - Wired SFW hard-drop into workflows (before trace validation).
  - Parsed `view_mode` from doc job inputs (case-insensitive allowlist; invalid values fail with `invalid_job_inputs`) and applied to `QueryPlan.filters.view_mode`.
- Frontend:
  - Added `ViewModeToggle` + localStorage persistence (`handshake.view_mode`).
  - `createJob(...)` attaches `view_mode` into doc job inputs (`doc_edit`, `doc_summarize`, `doc_rewrite`) when not already provided.
- Tests:
  - Added Rust integration tests for default NSFW + SFW hard-drop + guard behavior.

## HYGIENE
- Commands executed (see `## EVIDENCE` for outputs/log snippets):
  - `just hard-gate-wt-001`
  - `just pre-work WP-1-Lens-ViewMode-v1`
  - `just gov-check`
  - `just fmt`
  - `cd app; pnpm install --frozen-lockfile`
  - `just lint`
  - `cd app; pnpm run lint`
  - `cd app; pnpm test`
  - `just test`
  - `cd src/backend/handshake_core; cargo test`
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --target-dir "../Handshake Artifacts/handshake-cargo-target-release" --release --test lens_viewmode_tests -q`

## VALIDATION
- (Mechanical manifest for audit. This section records hashes/lines for Validator review. It is NOT a claim of official Validation.)
- **Target File**: `app/src/App.css`
- **Start**: 59
- **End**: 108
- **Line Delta**: 50
- **Pre-SHA1**: `2c0c011c75e7b09edbb0d07b4b7ee31554bf2468`
- **Post-SHA1**: `7a87bee83ac5d5d2aa91f03ec5d1edc8d2c7e189`
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

- **Target File**: `app/src/App.tsx`
- **Start**: 3
- **End**: 109
- **Line Delta**: 11
- **Pre-SHA1**: `4ce4a3c6791a8a371882e19e0e09c1c8d2789614`
- **Post-SHA1**: `80b9d2a1734b3f30079e1823ca0385a773259e5a`
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

- **Target File**: `app/src/components/ViewModeToggle.tsx`
- **Start**: 1
- **End**: 31
- **Line Delta**: 31
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `c73af54dd33028fe30f5b849c721df3fc6ce9df9`
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

- **Target File**: `app/src/lib/api.ts`
- **Start**: 1
- **End**: 645
- **Line Delta**: 18
- **Pre-SHA1**: `9614f5eecb99ed45d2cd26b41850b446101ac3b0`
- **Post-SHA1**: `2e131ab7051b1d18f304d767cf4cd92234fa9898`
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

- **Target File**: `app/src/lib/viewMode.ts`
- **Start**: 1
- **End**: 29
- **Line Delta**: 29
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `40ced7947df19db838e1d700f440a3bb5ed4bda2`
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

- **Target File**: `src/backend/handshake_core/src/ace/mod.rs`
- **Start**: 312
- **End**: 1132
- **Line Delta**: 110
- **Pre-SHA1**: `effe2e46c1e514ac8ec8a5a1626903316fba085a`
- **Post-SHA1**: `68b097f0aa3cec8b1b2f0b39acd113722886440c`
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
- **Start**: 3
- **End**: 1234
- **Line Delta**: 86
- **Pre-SHA1**: `25b80de87f0e8ae1c028b55e092a0e53ee008375`
- **Post-SHA1**: `24a27e5e87374878f4c30090cb471529c64e98f7`
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
- **Start**: 13
- **End**: 11359
- **Line Delta**: 27
- **Pre-SHA1**: `0fb7b3b8c92e0fe1454bea5a5664146af6644aca`
- **Post-SHA1**: `2c8260d8881fb89aeb38412a5336930be264d484`
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

- **Target File**: `src/backend/handshake_core/tests/lens_viewmode_tests.rs`
- **Start**: 1
- **End**: 186
- **Line Delta**: 186
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `65ad1cbc66873d97bdb5f0522b41373f17372061`
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
- Current WP_STATUS: IN_PROGRESS
- Touched files:
  - `src/backend/handshake_core/src/ace/mod.rs`
  - `src/backend/handshake_core/src/ace/validators/mod.rs`
  - `src/backend/handshake_core/src/workflows.rs`
  - `src/backend/handshake_core/tests/lens_viewmode_tests.rs`
  - `app/src/App.tsx`
  - `app/src/App.css`
  - `app/src/lib/api.ts`
  - `app/src/lib/viewMode.ts`
  - `app/src/components/ViewModeToggle.tsx`
- What changed in this update:
  - End-to-end ViewMode plumbing (UI toggle -> `job_inputs.view_mode` -> `QueryPlan.filters.view_mode` -> `RetrievalTrace.filters_applied`).
  - SFW strict-drop enforced via `apply_view_mode_hard_drop()` + `ViewModeHardDropGuard`.
  - Required SFW projection labeling fields added to serialized trace evidence (Addendum 11.3).
- Next step / handoff hint:
  - Run `just post-work WP-1-Lens-ViewMode-v1 --range 13efad9235ec5e7cfe4870bcb986b81d30416aed..HEAD`.

## EVIDENCE_MAPPING
- (Coder appends proof that DONE_MEANS + SPEC_ANCHOR requirements exist in code/tests. No verdicts.)
- Format (repeat as needed):
  - REQUIREMENT: "<quote DONE_MEANS bullet or SPEC_ANCHOR requirement>"
  - EVIDENCE: `path/to/file:line`
- REQUIREMENT: "ViewMode type exists per spec and is plumbed through Lens query inputs as \"NSFW\"|\"SFW\" (default \"NSFW\")."
  - EVIDENCE: `app/src/lib/viewMode.ts:1`
  - EVIDENCE: `app/src/lib/api.ts:640`
  - EVIDENCE: `src/backend/handshake_core/src/workflows.rs:2535`
  - EVIDENCE: `src/backend/handshake_core/src/ace/mod.rs:315`
- REQUIREMENT: "In ViewMode=\"SFW\", retrieval enforces strict drop (default-deny): no returned candidates/results with content_tier != \"sfw\"; content_tier==None is treated as non-sfw and dropped (no \"collapsed/blurred but revealable\" leakage)."
  - EVIDENCE: `src/backend/handshake_core/src/ace/mod.rs:593`
  - EVIDENCE: `src/backend/handshake_core/src/ace/mod.rs:796`
  - EVIDENCE: `src/backend/handshake_core/tests/lens_viewmode_tests.rs:20`
- REQUIREMENT: "SFW mode applies projection at render/output only and MUST NOT write back or mutate stored raw/derived descriptors/artifacts."
  - EVIDENCE: `src/backend/handshake_core/src/ace/mod.rs:796`
  - EVIDENCE: `src/backend/handshake_core/src/workflows.rs:2538`
- REQUIREMENT: "In ViewMode=\"SFW\", any SFW-projected output is labeled per spec Addendum 11.3 (required): projection_applied=true, projection_kind=\"SFW\", projection_ruleset_id, and a link to underlying raw evidence."
  - EVIDENCE: `src/backend/handshake_core/src/ace/mod.rs:753`
  - EVIDENCE: `src/backend/handshake_core/tests/lens_viewmode_tests.rs:137`
  - EVIDENCE: `src/backend/handshake_core/tests/lens_viewmode_tests.rs:112`
- REQUIREMENT: "QueryPlan and RetrievalTrace record ViewMode as a metadata filter (e.g., QueryPlan.filters.view_mode and/or trace fields), and serialized trace evidence includes it."
  - EVIDENCE: `src/backend/handshake_core/src/ace/mod.rs:345`
  - EVIDENCE: `src/backend/handshake_core/src/ace/mod.rs:750`
  - EVIDENCE: `src/backend/handshake_core/tests/lens_viewmode_tests.rs:133`
- REQUIREMENT: "Tests cover SFW hard-drop + trace recording (Rust unit tests; optional frontend test for toggle state/param)."
  - EVIDENCE: `src/backend/handshake_core/tests/lens_viewmode_tests.rs:20`
  - EVIDENCE: `src/backend/handshake_core/src/ace/validators/mod.rs:520`

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `<paste>`
  - EXIT_CODE: `<int>`
  - LOG_PATH: `.handshake/logs/WP-1-Lens-ViewMode-v1/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`
- COMMAND: `just gov-check`
  - EXIT_CODE: 0
  - PROOF_LINES:
    - `SPEC_CURRENT ok: Handshake_Master_Spec_v02.137.md`
    - `gov-check ok`
- COMMAND: `just fmt`
  - EXIT_CODE: 0
  - PROOF_LINES:
    - `cd src/backend/handshake_core; cargo fmt`
- COMMAND: `cd app; pnpm run lint`
  - EXIT_CODE: 0
  - PROOF_LINES:
    - `eslint src --ext .ts,.tsx`
- COMMAND: `cd app; pnpm test`
  - EXIT_CODE: 0
  - PROOF_LINES:
    - `Test Files  6 passed (6)`
    - `Tests       13 passed (13)`
- COMMAND: `just test`
  - EXIT_CODE: 1
  - PROOF_LINES:
    - `The paging file is too small for this operation to complete. (os error 1455)`
    - `crate \`libduckdb_sys\` required to be available in rlib format, but was not found in this form`
- COMMAND: `cd src/backend/handshake_core; cargo test`
  - EXIT_CODE: 0
  - PROOF_LINES:
    - `running 183 tests`
    - `Doc-tests handshake_core`
- COMMAND: `just test`
  - EXIT_CODE: 0
  - PROOF_LINES:
    - `running 183 tests`
    - `test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;`
- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --target-dir \"../Handshake Artifacts/handshake-cargo-target-release\" --release --test lens_viewmode_tests -q`
  - EXIT_CODE: 0
  - PROOF_LINES:
    - `running 3 tests`
    - `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
