# PROJECT_INVARIANTS

Project-specific invariants for Governance Pack instantiation (spec §7.5.4.9).

**Status:** ACTIVE  
**Updated:** 2026-02-05  

---

## 1) Identity

- PROJECT_CODE: HSK
- PROJECT_DISPLAY_NAME: Handshake
- ISSUE_PREFIX: HSK

## 2) Naming policy

- MASTER_SPEC_PATTERN: `Handshake_Master_Spec_vNN.NNN.md` (repo root)
- CODEX_FILENAME: `.GOV/codex/Handshake_Codex_v1.4.md` (repo root)
- CODEX_PATTERN: `Handshake Codex vX.Y.md` (repo root)

## 3) Canonical governance workspace (repo)

- GOVERNANCE_ROOT: `.GOV/`
- ROLE_PROTOCOLS_DIR: `.GOV/roles/`
- NAV_PACK_DIR: `.GOV/roles_shared/`
- WORK_PACKETS_LOGICAL_DIR: `.GOV/work_packets/` (logical resolver name; do not hard-code)
- TASK_PACKETS_DIR: `.GOV/task_packets/` (current physical storage root during compatibility migration)
- WORK_PACKET_ARCHIVE_DIR: `.GOV/task_packets/_archive/` (reserved physical archive root during lifecycle-layout migration)
- WORK_PACKET_SUPERSEDED_ARCHIVE_DIR: `.GOV/task_packets/_archive/superseded/`
- WORK_PACKET_VALIDATED_CLOSED_ARCHIVE_DIR: `.GOV/task_packets/_archive/validated_closed/`
- PACKET_RESOLVER_AUTHORITY: `.GOV/roles_shared/scripts/lib/runtime-paths.mjs`
- PACKET_CANONICAL_LAYOUT: logical `.GOV/work_packets/WP-{ID}/packet.md`; current physical storage `.GOV/task_packets/WP-{ID}/packet.md`; `.GOV/task_packets/WP-{ID}.md` remains legacy flat compatibility
- REFINEMENT_CANONICAL_LAYOUT: logical `.GOV/work_packets/WP-{ID}/refinement.md`; current physical storage `.GOV/task_packets/WP-{ID}/refinement.md`; `.GOV/refinements/WP-{ID}.md` remains legacy compatibility and pre-packet staging
- REFINEMENTS_DIR: `.GOV/refinements/` (legacy compatibility / pre-packet staging)
- TEMPLATES_DIR: `.GOV/templates/`
- GATES_STATE:
  - Orchestrator: `../gov_runtime/roles_shared/ORCHESTRATOR_GATES.json`
  - Validator: `../gov_runtime/roles_shared/validator_gates/{WP_ID}.json`
- ROLE_MAILBOX_EXPORT_DIR: `.GOV/roles_shared/exports/role_mailbox/`

## 4) Compatibility bundle (repo, temporary)

- COMPAT_DOCS_DIR: `docs/` (non-authoritative; compatibility only until product remediation lands)

## 5) Layout profile (Handshake repo)

- FRONTEND_ROOT_DIR: `app/`
- FRONTEND_SRC_DIR: `app/src/`
- BACKEND_CRATE_DIR: `src/backend/handshake_core/`
- BACKEND_SRC_DIR: `src/backend/handshake_core/src/`
- BACKEND_STORAGE_DIR: `src/backend/handshake_core/src/storage/`
- BACKEND_LLM_DIR: `src/backend/handshake_core/src/llm/`

## 6) Tooling paths (Handshake defaults)

- BUILD_ARTIFACTS_ROOT_DIR (external): `../Handshake_Artifacts/`
- CARGO_TARGET_DIR (external): `../Handshake_Artifacts/handshake-cargo-target`
- BUILD_ARTIFACTS_CANONICAL_DIRS: `handshake-cargo-target/`, `handshake-product/`, `handshake-test/`, `handshake-tool/`
- ARTIFACT_RETENTION_MANIFEST_DIR: `../Handshake_Artifacts/handshake-tool/artifact-retention/`
- BUILD_ARTIFACTS_POLICY: repo-local `target/` directories are invalid and must be cleaned or blocked by governance checks
- NODE_PACKAGE_MANAGER: `pnpm` (for `app/`)

## 7) Product runtime paths (Handshake defaults)

- PRODUCT_RUNTIME_ROOT_DIR (external default): `gov_runtime/`
- LEGACY_REPO_RUNTIME_DIRS (transitional): `data/`, `.handshake/`

## 8) Product-owned runtime governance state (default)

- RUNTIME_GOV_STATE_DIR: `.handshake/gov/` (configurable; runtime governance state only)
