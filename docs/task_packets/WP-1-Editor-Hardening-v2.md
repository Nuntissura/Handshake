# Task Packet: WP-1-Editor-Hardening-v2

## METADATA
- TASK_ID: WP-1-Editor-Hardening-v2
- WP_ID: WP-1-Editor-Hardening-v2
- BASE_WP_ID: WP-1-Editor-Hardening (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-01-16T22:18:03.313Z
- REQUESTOR: Operator (ilja)
- AGENT_ID: CodexCLI-GPT-5.2 (Orchestrator)
- ROLE: Orchestrator
- CODER_MODEL: GPT-5.2 (Codex CLI)
- CODER_REASONING_STRENGTH: HIGH
- **Status:** In Progress
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja160120262314

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-Editor-Hardening-v2.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Harden the document editor (Tiptap/BlockNote) and canvas editor (Excalidraw) integration so that all editor-triggered persistence conforms to the shared workspace model and the "No Silent Edits" invariant. Specifically, ensure AI-authored mutations are recorded as AI (`last_actor_kind=AI`) with non-null `job_id` and `workflow_id`, and that missing-context AI writes are rejected deterministically with `HSK-403-SILENT-EDIT` and surfaced to the Operator.
- Why: The Master Spec requires tool integrations to be projections over the same workspace model (no shadow pipelines) and requires mutation traceability with a hard StorageGuard at the persistence boundary. Editor surfaces are high-risk because they are primary mutation entry points.
- IN_SCOPE_PATHS:
  - app/src/components/DocumentView.tsx
  - app/src/components/ExcalidrawCanvas.tsx
  - app/src/components/CanvasView.tsx
  - app/src/lib/api.ts
  - src/backend/handshake_core/src/api/workspaces.rs
  - src/backend/handshake_core/src/api/canvases.rs
  - src/backend/handshake_core/src/api/jobs.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/tests.rs
- OUT_OF_SCOPE:
  - scripts/** and .github/** (reserved for governance/kernel conformance WP; avoid overlap)
  - docs/TASK_BOARD.md and docs/WP_TRACEABILITY_REGISTRY.md (Orchestrator-only coordination)
  - Any edits to historical task packets in docs/task_packets/*.md (locked history; create a new WP variant if scope changes)
  - Any new storage schema/table that introduces tool-specific persistence for docs/canvas (must reuse existing workspace entities)

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Editor-Hardening-v2

# Frontend
pnpm -C app run lint
pnpm -C app test

# Backend
cargo test --manifest-path src/backend/handshake_core/Cargo.toml

# Quick regression checks (expected NO matches after fixes in write paths for AI contexts):
rg -n \"WriteContext::human\\(None\\)\" src/backend/handshake_core/src/api

just cargo-clean
just post-work WP-1-Editor-Hardening-v2
```

### DONE_MEANS
- Editor-triggered AI mutations persist MutationMetadata as AI: `last_actor_kind=AI` and non-null `last_job_id` + `last_workflow_id` for affected rows (blocks/canvases/canvas_nodes/canvas_edges/workspaces/documents as applicable).
- Missing-context AI writes are rejected deterministically with `HSK-403-SILENT-EDIT` and the error is surfaced (API + UI/diagnostics) without silently downgrading the actor kind to HUMAN.
- Tool integration remains tool-agnostic: editors mutate the same workspace entities (no tool-specific storage schema or shadow write path).
- The TEST_PLAN passes, including `just post-work WP-1-Editor-Hardening-v2`.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.112.md (recorded_at: 2026-01-16T22:18:03.313Z)
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.112.md 2.2.0 + 2.9.3
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md
- WP Traceability: docs/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- If this is not a revision packet, write: `N/A`.

## BOOTSTRAP
- FILES_TO_OPEN:
  - docs/START_HERE.md
  - docs/SPEC_CURRENT.md
  - Handshake_Master_Spec_v02.112.md (2.2.0, 2.9.3)
  - docs/refinements/WP-1-Editor-Hardening-v2.md
  - app/src/components/DocumentView.tsx
  - app/src/components/ExcalidrawCanvas.tsx
  - src/backend/handshake_core/src/api/workspaces.rs
  - src/backend/handshake_core/src/api/canvases.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/tests.rs
- SEARCH_TERMS:
  - "WriteContext::human(None)"
  - "WriteContext::ai"
  - "HSK-403-SILENT-EDIT"
  - "last_actor_kind"
  - "last_job_id"
  - "last_workflow_id"
  - "edit_event_id"
  - "StorageGuard"
- RUN_COMMANDS:
  ```bash
  rg -n \"WriteContext::human\\(None\\)\" src/backend/handshake_core/src/api
  rg -n \"HSK-403-SILENT-EDIT|StorageGuard|MutationMetadata|last_actor_kind|last_job_id|last_workflow_id|edit_event_id\" src/backend/handshake_core/src
  pnpm -C app test
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  ```
- RISK_MAP:
  - "AI writes misclassified as HUMAN" -> "silent edit / audit failure"
  - "Over-broad API accepts spoofed AI context" -> "integrity + policy bypass"
  - "Editor path bypasses shared workspace model" -> "shadow pipeline and inconsistent behavior"
  - "Breaking editor persistence" -> "data loss / user-facing failures"

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
