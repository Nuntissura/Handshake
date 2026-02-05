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
- **Status:** Done
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja160120262314

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/WP-1-Editor-Hardening-v2.md
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
  - .GOV/scripts/** and .github/** (reserved for governance/kernel conformance WP; avoid overlap)
  - .GOV/roles_shared/TASK_BOARD.md and .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md (Orchestrator-only coordination)
  - Any edits to historical task packets in .GOV/task_packets/*.md (locked history; create a new WP variant if scope changes)
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
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.112.md 2.2.0 + 2.9.3
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- If this is not a revision packet, write: `N/A`.

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - Handshake_Master_Spec_v02.112.md (2.2.0, 2.9.3)
  - .GOV/refinements/WP-1-Editor-Hardening-v2.md
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
  - Frontend `app/src/lib/api.ts`: extend `request()` to accept extra headers; introduce a small `WriteContext` shape for write calls:
    - `actor_kind: "HUMAN" | "AI" | "SYSTEM"` (default to `"HUMAN"` when omitted)
    - `actor_id?: string`
    - `job_id?: string` (UUID; required when `actor_kind === "AI"`)
    - `workflow_id?: string` (UUID; required when `actor_kind === "AI"`; this is the WorkflowRun id returned by `createJob`)
  - Frontend write APIs:
    - `updateDocumentBlocks(documentId, blocks, ctx?: WriteContext)` sets headers on the PUT `/documents/:id/blocks` call.
    - `updateCanvasGraph(canvasId, nodes, edges, ctx?: WriteContext)` sets headers on the PUT `/canvases/:id` call.
    - (Optional for parity) `createWorkspace/createDocument/createCanvas/deleteDocument/deleteCanvas/deleteWorkspace` accept `ctx?: WriteContext` and forward headers, but default behaviour remains human.
  - Backend write-context extraction (in `src/backend/handshake_core/src/api/workspaces.rs` + `src/backend/handshake_core/src/api/canvases.rs`):
    - Add `headers: HeaderMap` extractor to write handlers.
    - Add a small helper `write_context_from_headers(&HeaderMap) -> Result<WriteContext, StorageError>` (or `Result<WriteContext, (StatusCode, Json<ErrorResponse>)>` per-handler) that:
      - Reads `x-hsk-actor-kind`, `x-hsk-actor-id`, `x-hsk-job-id`, `x-hsk-workflow-id`.
      - If `x-hsk-actor-kind == "AI"`: constructs `WriteContext::ai(...)` and does **not** silently downgrade if IDs are missing (let the guard reject deterministically).
      - Otherwise constructs a HUMAN or SYSTEM context (no job/workflow IDs).
    - Replace all `WriteContext::human(None)` in API write paths with derived ctx (this is required by the packet regression grep).
  - Backend AI-context sanity (to satisfy the "valid AI job" expectation in Spec 2.9.3):
    - When `actor_kind == AI`, validate `job_id` is parseable and resolves via `state.storage.get_ai_job(job_id_str)`; validate `job.workflow_run_id == workflow_id` when both exist.
    - On mismatch/not-found/invalid, return a deterministic `403` with `HSK-403-SILENT-EDIT` (no downgrade to HUMAN).

- API contract changes (editor write endpoints):
  - New optional headers accepted on editor persistence endpoints:
    - `x-hsk-actor-kind: HUMAN|AI|SYSTEM`
    - `x-hsk-actor-id: <string>` (optional)
    - `x-hsk-job-id: <uuid>` (required iff actor-kind is AI)
    - `x-hsk-workflow-id: <uuid>` (required iff actor-kind is AI)
  - Error contract (already implemented in API mappers):
    - If storage guard blocks an AI write without required context, respond `403` with JSON `{ "error": "HSK-403-SILENT-EDIT" }`.

- Where `HSK-403-SILENT-EDIT` is enforced end-to-end:
  1. Frontend calls `updateDocumentBlocks` / `updateCanvasGraph` with `WriteContext` headers (or defaults to HUMAN when omitted).
  2. Backend handler derives a `WriteContext` (HUMAN/AI/SYSTEM) from headers and passes it into storage methods.
  3. Storage methods call `StorageGuard::validate_write(ctx, resource_id)` and persist `MutationMetadata` (`last_actor_kind`, `last_job_id`, `last_workflow_id`, `edit_event_id`).
  4. `DefaultStorageGuard` rejects AI writes with missing `job_id`/`workflow_id` by emitting `StorageError::Guard("HSK-403-SILENT-EDIT")`.
  5. API `map_storage_error` in `workspaces.rs` / `canvases.rs` maps that to HTTP 403 + `{error:"HSK-403-SILENT-EDIT"}`.
  6. UI catches the 403 and surfaces it (inline error + optional `createDiagnostic({ code: "HSK-403-SILENT-EDIT", ... })` for Operator visibility).

- Open questions:
  - Frontend source of `workflow_id` for AI writes: use `createJob()` response `WorkflowRun.id` (store alongside `job_id`), or fetch via `getJob(job_id)` to obtain `workflow_run_id`.
  - Header naming: keep `x-hsk-*` vs `x-handshake-*` (no existing precedent found in repo; must pick one and use consistently).
  - Actor identity: is `actor_id` required/meaningful yet, or should it remain unset for HUMAN until auth/session exists?

- Notes:
  - Traceability gate resolved; Validator replied `SKELETON APPROVED`.

## IMPLEMENTATION
- Backend: derive `WriteContext` from `x-hsk-*` headers in `src/backend/handshake_core/src/api/workspaces.rs` and `src/backend/handshake_core/src/api/canvases.rs`; AI writes require valid `job_id` + `workflow_id` and never downgrade to HUMAN.
- Backend: map guard/missing-context to HTTP 403 `{ "error": "HSK-403-SILENT-EDIT" }` for deterministic enforcement.
- Storage: persist `MutationMetadata` onto parent rows (documents, canvases) in `src/backend/handshake_core/src/storage/sqlite.rs` and `src/backend/handshake_core/src/storage/postgres.rs` so editor-triggered writes update `last_actor_kind/last_job_id/last_workflow_id/edit_event_id/updated_at`.
- Frontend: add `WriteContext` + header plumbing in `app/src/lib/api.ts`; update `updateDocumentBlocks` / `updateCanvasGraph` to accept optional context and forward `x-hsk-*` headers.
- UI: surface `HSK-403-SILENT-EDIT` save failures via `createDiagnostic` in `app/src/components/DocumentView.tsx` and `app/src/components/CanvasView.tsx`.
- Tests: add targeted backend tests in `src/backend/handshake_core/src/api/workspaces.rs` for missing AI context -> 403 and valid AI context -> persisted metadata.

## HYGIENE
- Followed packet TEST_PLAN commands (see `## EVIDENCE` for outputs).
- Kept new SQLite parent-row UPDATEs as runtime `sqlx::query(...).bind(...)` to avoid `SQLX_OFFLINE` query-macro metadata requirements.

## VALIDATION
- Spec Target Resolved: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.112.md
- Operator: ilja

- **Target File**: `app/src/components/CanvasView.tsx`
- **Start**: 8
- **End**: 952
- **Line Delta**: 14
- **Pre-SHA1**: `651c81fba4e4d4841cd6fa7754b72ea1a0d77c6d`
- **Post-SHA1**: `65bd0ba432f00018050862f9301a1d74a699a964`
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

- **Target File**: `app/src/components/DocumentView.tsx`
- **Start**: 9
- **End**: 146
- **Line Delta**: 16
- **Pre-SHA1**: `c85bd153c5ed336f4a14ebaccc2c54c94e09050a`
- **Post-SHA1**: `654169a94269ddfc4e890a38c3fac0bf32badb82`
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
- **Start**: 6
- **End**: 417
- **Line Delta**: 28
- **Pre-SHA1**: `d83e63ea14721b7013620dfe0350b3370db9134d`
- **Post-SHA1**: `b84e88acc12f7ca198925b16c55b234a0541ca9a`
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

- **Target File**: `src/backend/handshake_core/src/api/canvases.rs`
- **Start**: 3
- **End**: 238
- **Line Delta**: 80
- **Pre-SHA1**: `db28d0d90f0d1c1bed64b9ae222669b19b74d0da`
- **Post-SHA1**: `94aaa26348bb7c49fc9df920129cb6dfc9b5a5e7`
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

- **Target File**: `src/backend/handshake_core/src/api/workspaces.rs`
- **Start**: 3
- **End**: 619
- **Line Delta**: 314
- **Pre-SHA1**: `31e951450c1ebceda3d27bc068e99150399fbb63`
- **Post-SHA1**: `08029f76c3e5fea6f38137e9e1192e6810b8dd35`
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

- **Target File**: `src/backend/handshake_core/src/storage/postgres.rs`
- **Start**: 713
- **End**: 1049
- **Line Delta**: 70
- **Pre-SHA1**: `58925914acaedf65f51ac7ad57ec5ccc537bde34`
- **Post-SHA1**: `e96e79bbeda0c49d4f80279d852c67bdc4e077c2`
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

- **Target File**: `src/backend/handshake_core/src/storage/sqlite.rs`
- **Start**: 779
- **End**: 1213
- **Line Delta**: 70
- **Pre-SHA1**: `ce22857f61a116397666d8e6792a6770d3d2b3c2`
- **Post-SHA1**: `52cc689c18200dcf93ae23a08c6fb2f8a5edad95`
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
- Current WP_STATUS: Implementation complete; ready for Validator validation.
- What changed in this update:
  - Editor write paths now pass header-derived `WriteContext` (HUMAN/AI/SYSTEM) into storage.
  - Deterministic rejection for AI writes missing/invalid context via `HSK-403-SILENT-EDIT` (API + UI diagnostics).
  - Parent entities (documents/canvases) now receive `MutationMetadata` updates on editor-triggered persistence.
- Next step / handoff hint:
  - Validator: review using `## VALIDATION` manifest + `## EVIDENCE` outputs; confirm DONE_MEANS.

## EVIDENCE
- Command: just pre-work WP-1-Editor-Hardening-v2
  Exit code: 0

- Command: rg -n "WriteContext::human\\(None\\)" src/backend/handshake_core/src/api
  Output: (no matches)

- Command: pnpm -C app run lint
  Exit code: 0

- Command: pnpm -C app test
  Exit code: 0
  Summary: Test Files: 5; Tests: 8

- Command: cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  Exit code: 0

- Command: just cargo-clean
  Exit code: 0

- Command: just post-work WP-1-Editor-Hardening-v2
  Exit code: 0

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)

### VALIDATION REPORT - WP-1-Editor-Hardening-v2 (2026-01-17)
Verdict: PASS

Scope Inputs:
- Task Packet: `.GOV/task_packets/WP-1-Editor-Hardening-v2.md`
- Spec Target: `.GOV/roles_shared/SPEC_CURRENT.md` -> `Handshake_Master_Spec_v02.112.md` (anchors: 2.2.0, 2.9.3)
- Governance Reference: `Handshake Codex v1.4.md` (per `.GOV/roles_shared/SPEC_CURRENT.md`)
- Worktree/Branch: `D:\Projects\LLM projects\wt-WP-1-Editor-Hardening-v2` / `feat/WP-1-Editor-Hardening-v2`
- Commit validated: `26e0faaf03d21fd97a2473971766e31b84f2b0d1`

Files Checked:
- `app/src/lib/api.ts`
- `app/src/components/DocumentView.tsx`
- `app/src/components/CanvasView.tsx`
- `src/backend/handshake_core/src/api/workspaces.rs`
- `src/backend/handshake_core/src/api/canvases.rs`
- `src/backend/handshake_core/src/storage/mod.rs`
- `src/backend/handshake_core/src/storage/sqlite.rs`
- `src/backend/handshake_core/src/storage/postgres.rs`

Findings (requirements -> evidence):
- Tool integration principle (single workspace graph; no shadow writes) is preserved: editor persistence continues to hit the same `/documents/:id/blocks` and `/canvases/:id` workspace entities; no new tool-specific schema introduced (Spec 2.2.0).
- Traceability + silent edit block enforced at persistence boundary (Spec 2.9.3):
  - Missing/invalid AI context results in deterministic guard failure `HSK-403-SILENT-EDIT` (guard logic: `src/backend/handshake_core/src/storage/mod.rs:720-724`; header parsing keeps AI kind: `src/backend/handshake_core/src/api/workspaces.rs:77-83`, `src/backend/handshake_core/src/api/canvases.rs:79-85`).
  - Guard errors map to HTTP 403 + `{ "error": "HSK-403-SILENT-EDIT" }` (`src/backend/handshake_core/src/api/workspaces.rs:365-373`, `src/backend/handshake_core/src/api/canvases.rs:322-330`).
  - Parent entities receive MutationMetadata updates for editor-triggered writes:
    - Documents (SQLite): `src/backend/handshake_core/src/storage/sqlite.rs:779-815`
    - Documents (Postgres): `src/backend/handshake_core/src/storage/postgres.rs:713-749`
    - Canvases (SQLite): `src/backend/handshake_core/src/storage/sqlite.rs:1170-1204`
    - Canvases (Postgres): `src/backend/handshake_core/src/storage/postgres.rs:1010-1037`
- Frontend write-context plumbing exists (headers are forwarded on write calls):
  - Header builder: `app/src/lib/api.ts:18-25`
  - Document persistence forwards optional context: `app/src/lib/api.ts:396-406`
  - Canvas persistence forwards optional context: `app/src/lib/api.ts:408-418`
- UI surfaces the block with diagnostics on 403:
  - Document editor: `app/src/components/DocumentView.tsx:133-145`
  - Canvas editor: `app/src/components/CanvasView.tsx:120-131`
- Targeted backend tests exist to guard the new behavior:
  - Missing AI context -> 403: `src/backend/handshake_core/src/api/workspaces.rs:426-469`
  - Valid AI context -> metadata persisted: `src/backend/handshake_core/src/api/workspaces.rs:471-618`

Test Verification (validator-run spot checks):
- `just pre-work WP-1-Editor-Hardening-v2` -> PASS
- `pnpm -C app run lint` -> PASS
- `pnpm -C app test` -> PASS
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml` -> PASS
- `just codex-check` -> PASS

Storage DAL Audit (CX-DBP-VAL-010..014):
- No direct DB pool access was introduced in API handlers; queries remain in storage layer; `sqlx::query` usage in API is limited to `#[cfg(test)]` validation tests (`src/backend/handshake_core/src/api/workspaces.rs:571-615`).
- Dual-backend readiness preserved: storage changes implemented in both SQLite and Postgres modules (see files checked above).

REASON FOR PASS:
- Editor write paths now propagate/derive `WriteContext` and enforce the "No Silent Edits" invariant deterministically at the storage boundary, with traceability metadata persisted and a regression test that fails if the enforcement is removed (Spec 2.2.0, 2.9.3).

Closure Notes:
- Task Board + traceability registry MUST be updated on `main` after merge: move `WP-1-Editor-Hardening-v2` to `## Done` as `[VALIDATED]` (packet status is now Done).

