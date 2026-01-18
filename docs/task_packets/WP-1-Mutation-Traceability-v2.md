# Task Packet: WP-1-Mutation-Traceability-v2

## METADATA
- TASK_ID: WP-1-Mutation-Traceability-v2
- WP_ID: WP-1-Mutation-Traceability-v2
- BASE_WP_ID: WP-1-Mutation-Traceability
- DATE: 2026-01-18T15:34:23.740Z
- REQUESTOR: User
- AGENT_ID: CodexCLI-GPT-5.2 (Orchestrator)
- ROLE: Orchestrator
- CODER_MODEL: <unclaimed>
- CODER_REASONING_STRENGTH: <unclaimed> (LOW | MEDIUM | HIGH | EXTRA_HIGH)
- **Status:** Ready for Dev
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja180120261630

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-Mutation-Traceability-v2.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Remediate/revalidate Mutation Traceability + StorageGuard enforcement to align with Master Spec v02.113 (2.9.3-2.9.3.2), including persistence schema fields and silent-edit rejection behavior.
- Why: Enforce "No Silent Edits" invariant: AI-authored writes must carry job/workflow context and persisted mutation metadata; missing-context AI writes are rejected deterministically with `HSK-403-SILENT-EDIT`.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/models.rs
  - src/backend/handshake_core/migrations/
- OUT_OF_SCOPE:
  - Any changes to src/backend/handshake_core/src/capabilities.rs or src/backend/handshake_core/src/workflows.rs (owned by WP-1-Capability-SSoT-v2)
  - UI surfacing of mutation metadata (Phase 2)
  - Backfilling historical provenance beyond minimal defaults needed for schema validity

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Mutation-Traceability-v2

# Coder (development):
cd src/backend/handshake_core; cargo test
cd src/backend/handshake_core; cargo clippy --all-targets --all-features

# Coder (pre-commit deterministic gate):
just post-work WP-1-Mutation-Traceability-v2

# Validator (protocol gates):
just validator-spec-regression
just validator-dal-audit
just validator-error-codes
just validator-hygiene-full
```

### DONE_MEANS
- `MutationMetadata`, `WriteActor`, and `StorageGuard` align to Master Spec v02.113 2.9.3-2.9.3.2 (including `edit_event_id` generation and `HSK-403-SILENT-EDIT` on rejection).
- All content tables enumerated in v02.113 2.9.3.1 have the required columns: `last_actor_kind`, `last_actor_id`, `last_job_id`, `last_workflow_id`, `edit_event_id` (via migrations as needed).
- A database check constraint (or strict application logic) enforces: AI writes cannot persist without `last_job_id` (per `CHECK (last_actor_kind != 'AI' OR last_job_id IS NOT NULL)`).
- Every persistence method in the `Database` trait that mutates content calls `StorageGuard::validate_write(...)` and persists the returned metadata fields.
- AI-authored writes without required context fail with `HSK-403-SILENT-EDIT` (tests cover human vs AI vs system write scenarios).

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.113.md (recorded_at: 2026-01-18T15:34:23.740Z)
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md 2.9.3, 2.9.3.1, 2.9.3.2 (Mutation Traceability / Persistence Schema / Storage Guard Trait)
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md
- WP Traceability: docs/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- Prior packet: docs/task_packets/WP-1-Mutation-Traceability.md (historical; spec drift revalidation FAIL).
- Stub: docs/task_packets/stubs/WP-1-Mutation-Traceability-v2.md (planning stub; not executable).
- Preserved scope: Mutation traceability metadata + StorageGuard enforcement + silent-edit rejection.
- Updated in v2: re-anchored to Master Spec v02.113 (2.9.3-2.9.3.2) and revalidated schema column names/invariants + integration requirements.

## BOOTSTRAP
- FILES_TO_OPEN:
  - docs/START_HERE.md
  - docs/SPEC_CURRENT.md
  - docs/ARCHITECTURE.md
  - Handshake_Master_Spec_v02.113.md
  - docs/task_packets/WP-1-Mutation-Traceability.md
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/models.rs
  - src/backend/handshake_core/migrations/
- SEARCH_TERMS:
  - "MutationMetadata"
  - "WriteActor"
  - "StorageGuard"
  - "validate_write"
  - "HSK-403-SILENT-EDIT"
  - "last_actor_kind"
  - "last_job_id"
  - "last_workflow_id"
  - "edit_event_id"
- RUN_COMMANDS:
  ```bash
  cd src/backend/handshake_core; cargo test
  ```
- RISK_MAP:
  - "silent AI edit path not guarded" -> "safety invariant violated; audit bypass"
  - "schema mismatch between sqlite/postgres" -> "runtime failures or missing traceability data"
  - "missing edit_event_id generation/persistence" -> "traceability anchor absent; audit chain breaks"

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
