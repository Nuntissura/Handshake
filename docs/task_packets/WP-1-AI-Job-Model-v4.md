# Task Packet: WP-1-AI-Job-Model-v4

## METADATA
- TASK_ID: WP-1-AI-Job-Model-v4
- WP_ID: WP-1-AI-Job-Model-v4
- BASE_WP_ID: WP-1-AI-Job-Model (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-01-19T23:59:37.328Z
- REQUESTOR: ilja
- AGENT_ID: codex-cli
- ROLE: Orchestrator
- CODER_MODEL: GPT-5.2 (Codex CLI)
- CODER_REASONING_STRENGTH: HIGH
- **Status:** In Progress
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja200120260048

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-AI-Job-Model-v4.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Align the backend AI Job Model JobKind storage/parsing to Master Spec v02.113 (canonical JobKind strings + strict enum mapping), including alias normalization for terminal job kinds and removing non-spec persisted kinds.
- Why: Current implementation violates 2.6.6.2.8.1 by writing `term_exec` instead of canonical `terminal_exec`, and persists non-spec kinds (`doc_test`, `governance_pack_export`) which breaks the spec's strict "reject unknown kinds" invariant.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/migrations/0002_create_ai_core_tables.sql
  - src/backend/handshake_core/src/capabilities.rs
  - src/backend/handshake_core/src/api/jobs.rs
  - src/backend/handshake_core/src/api/governance_pack.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
- OUT_OF_SCOPE:
  - Spec enrichment (new Master Spec versions)
  - Frontend/UI changes (app/)
  - Unrelated workflow engine behavior changes beyond what is required to keep this WP's job_kind writes canonical

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-AI-Job-Model-v4

# Targeted backend checks
cd src/backend/handshake_core
cargo fmt
cargo test storage
cargo test workflows
cargo clippy --all-targets --all-features
cd ../../..

just cargo-clean
just post-work WP-1-AI-Job-Model-v4
```

### DONE_MEANS
- `JobKind` canonical strings match Master Spec v02.113 2.6.6.2.8.1 exactly; unknown values are rejected at parse time.
- `term_exec` is accepted only as an alias, and is normalized to `terminal_exec` on write (no new rows persisted with `term_exec`).
- Spec canonical kinds missing in code are supported (`doc_rewrite`, `spec_router`, `doc_ingest`, `distillation_eval`) with end-to-end storage parsing/serialization.
- Non-spec persisted kinds are removed/neutralized: `doc_test` and `governance_pack_export` are not persisted as `ai_jobs.job_kind` values.
- Existing DB rows are migrated to canonical `ai_jobs.job_kind` values (no stranded legacy strings).
- `just pre-work WP-1-AI-Job-Model-v4` and `just post-work WP-1-AI-Job-Model-v4` PASS.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.113.md (recorded_at: 2026-01-19T23:59:37.328Z)
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR:
  - Handshake_Master_Spec_v02.113.md 2.6.6.2.8 [HSK-JOB-100, HSK-JOB-101]
  - Handshake_Master_Spec_v02.113.md 2.6.6.2.8.1 (JobKind Canonical Strings, alias normalization)
  - Handshake_Master_Spec_v02.113.md 2.6.6.3 (Job lifecycle model; JobState set includes `stalled`)
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md
- WP Traceability: docs/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- Prior packets:
  - docs/task_packets/WP-1-AI-Job-Model-v2.md (historical; validated against old spec baseline)
  - docs/task_packets/WP-1-AI-Job-Model-v3.md (historical; later revalidation FAIL due to governance gates + spec drift)
  - docs/task_packets/stubs/WP-1-AI-Job-Model-v4.md (stub remediation pointer; now activated)
- Preserved requirements (carried forward):
  - [HSK-JOB-100] strict enum mapping via validated `FromStr` (reject illegal states)
  - [HSK-JOB-101] metrics integrity (no NULL metrics; zeroed at init)
  - JobState includes `stalled` and supports recovery semantics
- Changed in v4:
  - Re-anchor to SPEC_CURRENT v02.113 and enforce canonical JobKind strings + alias normalization on write.
  - Remove/neutralize persisted non-spec JobKinds introduced by other WPs (`doc_test`, `governance_pack_export`).

## BOOTSTRAP
- FILES_TO_OPEN:
  - docs/START_HERE.md
  - docs/SPEC_CURRENT.md
  - docs/ARCHITECTURE.md
  - Handshake_Master_Spec_v02.113.md
  - docs/refinements/WP-1-AI-Job-Model-v4.md
  - docs/task_packets/stubs/WP-1-AI-Job-Model-v4.md
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/migrations/0002_create_ai_core_tables.sql
  - src/backend/handshake_core/src/capabilities.rs
  - src/backend/handshake_core/src/api/jobs.rs
  - src/backend/handshake_core/src/api/governance_pack.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
- SEARCH_TERMS:
  - "enum JobKind"
  - "JobKind::TerminalExec"
  - "\"term_exec\""
  - "\"terminal_exec\""
  - "\"governance_pack_export\""
  - "\"doc_test\""
  - "doc_rewrite"
  - "spec_router"
  - "doc_ingest"
  - "distillation_eval"
- RUN_COMMANDS:
  ```bash
  just pre-work WP-1-AI-Job-Model-v4
  cd src/backend/handshake_core
  cargo test storage
  cargo test workflows
  ```
- RISK_MAP:
  - "job_kind canonicalization drift" -> "capability enforcement and audit mismatches"
  - "migration leaves legacy job_kind strings" -> "future strict parsing rejects old rows"
  - "removing non-spec job_kind breaks governance pack export" -> "governance export path fails until remapped to canonical kind"

## SKELETON
- Proposed interfaces/types/contracts:
  - `JobKind` enum variants and string mapping must match 2.6.6.2.8.1 canonical list.
  - `JobKind::from_str` must accept only canonical values (plus alias `term_exec`) and reject all other strings.
  - `JobKind::as_str` (or equivalent write path) must emit canonical strings only (normalize `term_exec` to `terminal_exec`).
- Open questions:
  - NONE (refinement resolved ambiguous non-spec kinds by requiring conformance to the canonical JobKind list without spec enrichment).
- Notes:
  - `governance_pack_export` currently exists as a JobKind in code but is not in the v02.113 canonical JobKind list; this WP requires eliminating it as a persisted `ai_jobs.job_kind` value.

SKELETON APPROVED

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
- What changed in this update: Bootstrap claimed (coder fields filled; status set to In Progress).
- Next step / handoff hint: Proceed with BOOTSTRAP -> SKELETON -> IMPLEMENTATION per docs/CODER_PROTOCOL.md.

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
