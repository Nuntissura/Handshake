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
- **Status:** Done
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja200120260048

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/WP-1-AI-Job-Model-v4.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Align the backend AI Job Model JobKind storage/parsing to Master Spec v02.113 (canonical JobKind strings + strict enum mapping), including alias normalization for terminal job kinds and removing non-spec persisted kinds.
- Why: Current implementation violates 2.6.6.2.8.1 by writing `term_exec` instead of canonical `terminal_exec`, and persists non-spec kinds (`doc_test`, `governance_pack_export`) which breaks the spec's strict "reject unknown kinds" invariant.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/migrations/0002_create_ai_core_tables.sql
  - src/backend/handshake_core/migrations/0010_normalize_ai_job_kind.sql
  - src/backend/handshake_core/migrations/0010_normalize_ai_job_kind.down.sql
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
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR:
  - Handshake_Master_Spec_v02.113.md 2.6.6.2.8 [HSK-JOB-100, HSK-JOB-101]
  - Handshake_Master_Spec_v02.113.md 2.6.6.2.8.1 (JobKind Canonical Strings, alias normalization)
  - Handshake_Master_Spec_v02.113.md 2.6.6.3 (Job lifecycle model; JobState set includes `stalled`)
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- Prior packets:
  - .GOV/task_packets/WP-1-AI-Job-Model-v2.md (historical; validated against old spec baseline)
  - .GOV/task_packets/WP-1-AI-Job-Model-v3.md (historical; later revalidation FAIL due to governance gates + spec drift)
  - .GOV/task_packets/stubs/WP-1-AI-Job-Model-v4.md (stub remediation pointer; now activated)
- Preserved requirements (carried forward):
  - [HSK-JOB-100] strict enum mapping via validated `FromStr` (reject illegal states)
  - [HSK-JOB-101] metrics integrity (no NULL metrics; zeroed at init)
  - JobState includes `stalled` and supports recovery semantics
- Changed in v4:
  - Re-anchor to SPEC_CURRENT v02.113 and enforce canonical JobKind strings + alias normalization on write.
  - Remove/neutralize persisted non-spec JobKinds introduced by other WPs (`doc_test`, `governance_pack_export`).

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - .GOV/roles_shared/ARCHITECTURE.md
  - Handshake_Master_Spec_v02.113.md
  - .GOV/refinements/WP-1-AI-Job-Model-v4.md
  - .GOV/task_packets/stubs/WP-1-AI-Job-Model-v4.md
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
- Updated JobKind string normalization to emit canonical values on write (accept alias `term_exec`, normalize to `terminal_exec`).
- Removed persisted non-spec job kinds (`doc_test`, `governance_pack_export`) and added missing canonical kinds per v02.113.
- Remapped governance-pack export to `workflow_run` + protocol ID (`hsk.governance_pack.export.v0`) and made capability gating protocol-aware for `workflow_run`.
- Added DB migration to normalize legacy persisted `ai_jobs.job_kind` strings to canonical kinds and to set `protocol_id` for legacy governance-pack export rows.

## HYGIENE
- Ran: `just pre-work WP-1-AI-Job-Model-v4`
- Ran: `cargo fmt`
- Ran: `cargo test storage`, `cargo test workflows`
- Ran: `cargo clippy --all-targets --all-features`
- Ran: `just validator-scan`, `just validator-dal-audit`, `just validator-git-hygiene`

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`.GOV/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `src/backend/handshake_core/src/storage/mod.rs`
- **Start**: 1
- **End**: 804
- **Line Delta**: 5
- **Pre-SHA1**: `74b087d7be135bf7567e2d2f891448758a8119c9`
- **Post-SHA1**: `44e6bc6f216dd439d624e9269b352b5f7eda1f2a`
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
- **End**: 517
- **Line Delta**: 80
- **Pre-SHA1**: `17a8cc61f381a5865cb2eefe686043a2ed65b34d`
- **Post-SHA1**: `9e2ab5c08a7279e0ef30864ecf02790c00179069`
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
- **End**: 1523
- **Line Delta**: 2
- **Pre-SHA1**: `9a65516f84419b3fe7f733641fd7f49e418930e4`
- **Post-SHA1**: `d81e1170997f846a0da1fee072c4fc310f902a68`
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

- **Target File**: `src/backend/handshake_core/src/api/governance_pack.rs`
- **Start**: 1
- **End**: 54
- **Line Delta**: 1
- **Pre-SHA1**: `f2ee0030f236da58db7d991fb741a676fedb9ba0`
- **Post-SHA1**: `97801f9384117de7a84072295154cd12c8f2f581`
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

- **Target File**: `src/backend/handshake_core/src/api/jobs.rs`
- **Start**: 1
- **End**: 251
- **Line Delta**: 0
- **Pre-SHA1**: `239e08bd953cae8e27d2f43528b1580491ad5df4`
- **Post-SHA1**: `49b8c6c3c6d7d83cb1a6b93245984b9eb4a3ea27`
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

- **Target File**: `src/backend/handshake_core/migrations/0010_normalize_ai_job_kind.sql`
- **Start**: 1
- **End**: 20
- **Line Delta**: 20
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `2f6166f1f1aac1b2215e5c86f1b1c185f09b28f0`
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

- **Target File**: `src/backend/handshake_core/migrations/0010_normalize_ai_job_kind.down.sql`
- **Start**: 1
- **End**: 3
- **Line Delta**: 3
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `a1d4b371a305ed7d56dfad37016019dccd69260a`
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

- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.113.md

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: In Progress
- What changed in this update: Canonicalized persisted `ai_jobs.job_kind` strings per v02.113; remapped governance-pack export to `workflow_run` + protocol ID; added migration for legacy normalization.
- Touched files:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/capabilities.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/api/governance_pack.rs
  - src/backend/handshake_core/src/api/jobs.rs
  - src/backend/handshake_core/migrations/0010_normalize_ai_job_kind.sql
  - src/backend/handshake_core/migrations/0010_normalize_ai_job_kind.down.sql
- Next step / handoff hint: Validator audit + merge when ready.

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Implementation commit: 054d2d39
- DONE_MEANS evidence:
  - Checked "JobKind canonical strings match spec; reject unknown strings" at src/backend/handshake_core/src/storage/mod.rs:354 and src/backend/handshake_core/src/storage/mod.rs:391
  - Checked "term_exec accepted as alias; normalized on write to terminal_exec" at src/backend/handshake_core/src/storage/mod.rs:373 and src/backend/handshake_core/src/storage/mod.rs:403
  - Checked "Missing canonical kinds supported (doc_rewrite/spec_router/doc_ingest/distillation_eval)" at src/backend/handshake_core/src/storage/mod.rs:356 and src/backend/handshake_core/src/storage/mod.rs:369
  - Checked "governance-pack export does not persist non-spec job_kind" at src/backend/handshake_core/src/api/governance_pack.rs:28 and src/backend/handshake_core/src/workflows.rs:947
  - Checked "Existing DB rows migrated to canonical kinds" at src/backend/handshake_core/migrations/0010_normalize_ai_job_kind.sql:8 and src/backend/handshake_core/migrations/0010_normalize_ai_job_kind.sql:16
  - Checked "Create-job terminal uses canonical kind (terminal_exec)" at src/backend/handshake_core/src/api/jobs.rs:221
- `just post-work WP-1-AI-Job-Model-v4` output (2026-01-20):
```text
Checking Phase Gate for WP-1-AI-Job-Model-v4...
? GATE PASS: Workflow sequence verified.

Post-work validation for WP-1-AI-Job-Model-v4 (deterministic manifest + gates)...

Check 1: Validation manifest present
warning: in the working copy of '.GOV/task_packets/WP-1-AI-Job-Model-v4.md', CRLF will be replaced by LF the next time Git touches it

Check 2: Manifest fields

Check 3: File integrity (per manifest entry)
fatal: path 'src/backend/handshake_core/migrations/0010_normalize_ai_job_kind.sql' exists on disk, but not in 'HEAD'
fatal: path 'src/backend/handshake_core/migrations/0010_normalize_ai_job_kind.down.sql' exists on disk, but not in 'HEAD'

Check 4: Git status
warning: in the working copy of '.GOV/task_packets/WP-1-AI-Job-Model-v4.md', CRLF will be replaced by LF the next time Git touches it

==================================================
Post-work validation PASSED with warnings

Warnings:
  1. Manifest[6]: Could not load HEAD version (new file or not tracked): src\backend\handshake_core\migrations\0010_normalize_ai_job_kind.sql
  2. Manifest[7]: Could not load HEAD version (new file or not tracked): src\backend\handshake_core\migrations\0010_normalize_ai_job_kind.down.sql

You may proceed with commit.
? ROLE_MAILBOX_EXPORT_GATE PASS
```

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)

### VALIDATOR_REPORT (2026-01-20)

VALIDATION REPORT - WP-1-AI-Job-Model-v4
Verdict: FAIL

Scope Inputs:
- Task Packet: .GOV/task_packets/WP-1-AI-Job-Model-v4.md (status: In Progress)
- Spec Target Resolved: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.113.md
- Spec anchors validated: Handshake_Master_Spec_v02.113.md 2.6.6.2.8 [HSK-JOB-100/101], 2.6.6.2.8.1 (JobKind Canonical Strings)

Files Checked:
- .GOV/task_packets/WP-1-AI-Job-Model-v4.md
- .GOV/refinements/WP-1-AI-Job-Model-v4.md
- .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md
- .GOV/roles_shared/TASK_BOARD.md
- Handshake_Master_Spec_v02.113.md
- src/backend/handshake_core/src/storage/mod.rs
- src/backend/handshake_core/src/api/governance_pack.rs
- src/backend/handshake_core/src/capabilities.rs
- src/backend/handshake_core/src/workflows.rs

Findings:
- Packet not implementation-ready: IMPLEMENTATION/HYGIENE/VALIDATION sections are still placeholders: .GOV/task_packets/WP-1-AI-Job-Model-v4.md:153, .GOV/task_packets/WP-1-AI-Job-Model-v4.md:156, .GOV/task_packets/WP-1-AI-Job-Model-v4.md:159.
- JobKind alias normalization (Spec): term_exec MAY be accepted as an alias for terminal_exec, but MUST be normalized to terminal_exec on write: Handshake_Master_Spec_v02.113.md:5389. Code still serializes TerminalExec as term_exec: src/backend/handshake_core/src/storage/mod.rs:379.
- Non-spec JobKinds persisted/accepted: Spec canonical list excludes doc_test and governance_pack_export (Handshake_Master_Spec_v02.113.md:5371-5389). Code still defines and accepts them: src/backend/handshake_core/src/storage/mod.rs:364, src/backend/handshake_core/src/storage/mod.rs:368, src/backend/handshake_core/src/storage/mod.rs:400, src/backend/handshake_core/src/storage/mod.rs:402.
- Governance pack export still modeled as non-spec job_kind: API creates JobKind::GovernancePackExport: src/backend/handshake_core/src/api/governance_pack.rs:27; capabilities map includes governance_pack_export: src/backend/handshake_core/src/capabilities.rs:159; workflows dispatch includes JobKind::GovernancePackExport: src/backend/handshake_core/src/workflows.rs:1131.
- Missing canonical kinds: Spec canonical list includes doc_rewrite/spec_router/doc_ingest/distillation_eval: Handshake_Master_Spec_v02.113.md:5374, Handshake_Master_Spec_v02.113.md:5380, Handshake_Master_Spec_v02.113.md:5383, Handshake_Master_Spec_v02.113.md:5384. These variants are not present in JobKind: src/backend/handshake_core/src/storage/mod.rs:354.

Deterministic Gates:
- just pre-work WP-1-AI-Job-Model-v4: PASS (packet/refinement gates only; no implementation diff required).
- just post-work WP-1-AI-Job-Model-v4: FAIL (No files changed), because there is no non-doc implementation diff on this branch.

Tests:
- Not run (no implementation changes present on feat/WP-1-AI-Job-Model-v4).

Risks & Suggested Actions:
- Implement the packet DONE_MEANS (JobKind canonical strings + alias normalization + remove non-spec persisted kinds + migration) and populate IMPLEMENTATION/HYGIENE/VALIDATION manifest.
- After implementation is staged, rerun just post-work WP-1-AI-Job-Model-v4 and provide evidence logs in ## EVIDENCE.

REASON FOR FAIL:
- Current feat/WP-1-AI-Job-Model-v4 contains only docs bootstrap/activation commits and does not implement the required spec-aligned JobKind changes; current code remains non-compliant with Handshake_Master_Spec_v02.113.md 2.6.6.2.8.1 canonical JobKind strings.

### VALIDATOR_REPORT (2026-01-20)

VALIDATION REPORT - WP-1-AI-Job-Model-v4
Verdict: PASS

Scope Inputs:
- Task Packet: .GOV/task_packets/WP-1-AI-Job-Model-v4.md (status: Done)
- Spec Target Resolved: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.113.md
- Spec anchors validated: Handshake_Master_Spec_v02.113.md:5369, Handshake_Master_Spec_v02.113.md:5389
- Commits validated: 054d2d39 (implementation), b3b4b0d7 (packet evidence)

Files Checked:
- .GOV/task_packets/WP-1-AI-Job-Model-v4.md
- .GOV/refinements/WP-1-AI-Job-Model-v4.md
- .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md
- .GOV/roles_shared/TASK_BOARD.md
- Handshake_Master_Spec_v02.113.md
- src/backend/handshake_core/src/storage/mod.rs
- src/backend/handshake_core/migrations/0010_normalize_ai_job_kind.sql
- src/backend/handshake_core/src/capabilities.rs
- src/backend/handshake_core/src/api/jobs.rs
- src/backend/handshake_core/src/api/governance_pack.rs
- src/backend/handshake_core/src/workflows.rs

Findings:
- JobKind canonical coverage + strict FromStr reject-unknown: src/backend/handshake_core/src/storage/mod.rs:354, src/backend/handshake_core/src/storage/mod.rs:391, src/backend/handshake_core/src/storage/mod.rs:408
- Alias acceptance + canonical normalization on write (term_exec -> terminal_exec): src/backend/handshake_core/src/storage/mod.rs:382, src/backend/handshake_core/src/storage/mod.rs:403; terminal job creation uses canonical kind: src/backend/handshake_core/src/api/jobs.rs:221
- Legacy row normalization migration (no stranded legacy strings): src/backend/handshake_core/migrations/0010_normalize_ai_job_kind.sql:8, src/backend/handshake_core/migrations/0010_normalize_ai_job_kind.sql:12, src/backend/handshake_core/migrations/0010_normalize_ai_job_kind.sql:16
- Governance pack export uses canonical workflow_run + protocol-aware gating/dispatch (no client protocol escalation): src/backend/handshake_core/src/api/governance_pack.rs:28, src/backend/handshake_core/src/capabilities.rs:306, src/backend/handshake_core/src/capabilities.rs:331, src/backend/handshake_core/src/workflows.rs:578, src/backend/handshake_core/src/workflows.rs:947

Hygiene:
- Forbidden pattern grep on touched prod files: clean (no unwrap/expect/todo!/panic!/dbg!/println!/eprintln!/split_whitespace/placeholder/hollow).

Storage DAL Audit:
- just validator-dal-audit: PASS

Tests (Validator-run):
- cargo fmt --check: PASS
- cargo test storage: PASS
- cargo test workflows: PASS
- cargo clippy --all-targets --all-features: PASS (warnings only; no deny)

Risks & Suggested Actions:
- Note: just post-work WP-1-AI-Job-Model-v4 is a pre-commit determinism gate; the pre-commit run output is recorded in ## EVIDENCE. Re-running it post-commit on a clean tree is expected to fail due to COR-701 preimage checks.

REASON FOR PASS:
- Implementation aligns JobKind storage/parsing with Handshake_Master_Spec_v02.113.md 2.6.6.2.8.1 (including term_exec alias normalization) and removes non-spec persisted kinds via deterministic migration, with tests + DAL audit passing.


