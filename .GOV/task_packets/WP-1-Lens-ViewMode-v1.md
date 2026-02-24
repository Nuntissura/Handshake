# Task Packet: WP-1-Lens-ViewMode-v1

## METADATA
- TASK_ID: WP-1-Lens-ViewMode-v1
- WP_ID: WP-1-Lens-ViewMode-v1
- BASE_WP_ID: WP-1-Lens-ViewMode (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-02-24T12:03:19.298Z
- MERGE_BASE_SHA: 35cd220dbfe573628ce1ab565a6363f0b993a1eb
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
just post-work WP-1-Lens-ViewMode-v1 --range 35cd220dbfe573628ce1ab565a6363f0b993a1eb..HEAD
```

### DONE_MEANS
- ViewMode type exists per spec and is plumbed through Lens query inputs as \"NSFW\"|\"SFW\" (default \"NSFW\").
- In ViewMode=\"SFW\", retrieval enforces strict drop: no returned candidates/results with content_tier != \"sfw\" (no \"collapsed/blurred but revealable\" leakage).
- SFW mode applies projection at render/output only and MUST NOT write back or mutate stored raw/derived descriptors/artifacts.
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
- Open questions:
- Notes:

## END_TO_END_CLOSURE_PLAN [CX-E2E-001]
- END_TO_END_CLOSURE_PLAN_APPLICABLE: YES
- TRUST_BOUNDARY: client->server (ViewMode toggle influences retrieval + output projection; server must enforce)
- SERVER_SOURCES_OF_TRUTH:
  - Stored artifact/descriptor governance fields (content_tier) are loaded from storage and MUST NOT be client-supplied.
  - ViewMode is validated server-side (enum allowlist); invalid values are rejected.
- REQUIRED_PROVENANCE_FIELDS:
  - view_mode (\"NSFW\"|\"SFW\")
  - content_tier (per returned item / candidate, when applicable)
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
  - SFW is projection-only (no storage mutation) and strict drop is proven via tests.

## IMPLEMENTATION
- (Coder fills after skeleton approval.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`.GOV/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
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
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_vXX.XX.md
- **Notes**:

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: IN_PROGRESS (bootstrap claim)
- What changed in this update: Coder claimed the WP; beginning SKELETON draft.
- Next step / handoff hint: Produce SKELETON checkpoint commit; wait for "SKELETON APPROVED" before implementation.

## EVIDENCE_MAPPING
- (Coder appends proof that DONE_MEANS + SPEC_ANCHOR requirements exist in code/tests. No verdicts.)
- Format (repeat as needed):
  - REQUIREMENT: "<quote DONE_MEANS bullet or SPEC_ANCHOR requirement>"
  - EVIDENCE: `path/to/file:line`

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `<paste>`
  - EXIT_CODE: `<int>`
  - LOG_PATH: `.handshake/logs/WP-1-Lens-ViewMode-v1/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
