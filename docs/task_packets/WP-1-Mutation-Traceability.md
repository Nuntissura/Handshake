# Work Packet: WP-1-Mutation-Traceability

**Status:** READY FOR DEV 🔴  
**Authority:** Master Spec §2.9.3 Mutation Traceability (No Silent Edits)  
**USER_SIGNATURE:** <pending>

---

## Executive Summary
Implement mutation traceability and storage guard to enforce “No Silent Edits”: all RAW mutations must carry actor/job/workflow metadata and be blocked for AI writes without approval context.

**Current State:**
- No enforced MutationMetadata fields; storage guard not implemented.

**End State:**
- MutationMetadata schema persisted (last_actor_kind/id, last_job_id, last_workflow_id, edit_event_id).
- StorageGuard trait implemented; AI writes without job/approval fail with typed error (HSK-403-SILENT-EDIT).
- Tests covering human/AI/system writes; storage guard enforced in mutation paths.

**Effort:** 10-14 hours  
**Phase 1 Blocking:** YES (Spec §2.9.3)

---

## Technical Contract (LAW)
Governed by Master Spec §2.9.3:
- Add MutationMetadata columns to content tables (blocks, canvas_nodes, canvas_edges, workspaces, documents).
- Implement StorageGuard trait; validate AI writes require job/approval context.
- Enforce check: if last_actor_kind == 'AI' then last_job_id NOT NULL.

---

## Scope
### In Scope
1) Schema changes (or logic) to persist MutationMetadata.
2) StorageGuard implementation; integrate into mutation paths.
3) Tests for human/AI/system writes, silent edit rejection, and valid metadata persistence.

### Out of Scope
- UI surfacing of metadata (Phase 2).
- Historical backfill of older edits (Phase 2).

---

## Quality Gate
- **RISK_TIER:** HIGH (Safety)
- **DONE_MEANS:**
  - MutationMetadata persisted per §2.9.3; guard enforces “No Silent Edits”.
  - AI writes without job/approval → typed error HSK-403-SILENT-EDIT.
  - Tests cover human/AI/system writes; guard applied to mutation handlers.
  - No forbidden patterns (unwrap/expect/panic/dbg/Value in domain).
- **TEST_PLAN:**
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`
  - `just validator-spec-regression`
  - `just validator-hygiene-full`
  - `just validator-error-codes`

---

## BOOTSTRAP
- **FILES_TO_OPEN:** docs/SPEC_CURRENT.md; Handshake_Master_Spec_v02.84.md (§2.9.3); src/backend/handshake_core/src/storage; src/backend/handshake_core/src/api; src/backend/handshake_core/src/workflows.rs
- **SEARCH_TERMS:** "MutationMetadata", "StorageGuard", "SilentEdit", "last_actor", "job_id", "workflow_id"
- **RUN_COMMANDS:** cargo test; just validator-spec-regression; just validator-hygiene-full
- **RISK_MAP:** "Silent edits -> safety breach"; "Missing metadata -> audit gap"; "Schema mismatch -> runtime failure"

---

## Success Metrics
| Metric | Target | Verification |
|--------|--------|--------------|
| Guard blocks AI without context | 100% | Unit/integration tests |
| Metadata persisted | All mutation paths | Tests + code evidence |
| Typed errors | HSK-403-SILENT-EDIT used | Code evidence |

---

**Last Updated:** 2025-12-25  
**User Signature Locked:** <pending>
