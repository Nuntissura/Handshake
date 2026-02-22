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
- CODEX_FILENAME: `Handshake Codex v1.4.md` (repo root)
- CODEX_PATTERN: `Handshake Codex vX.Y.md` (repo root)

## 3) Canonical governance workspace (repo)

- GOVERNANCE_ROOT: `.GOV/`
- ROLE_PROTOCOLS_DIR: `.GOV/roles/`
- NAV_PACK_DIR: `.GOV/roles_shared/`
- TASK_PACKETS_DIR: `.GOV/task_packets/`
- REFINEMENTS_DIR: `.GOV/refinements/`
- TEMPLATES_DIR: `.GOV/templates/`
- GATES_STATE:
  - Orchestrator: `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json`
  - Validator: `.GOV/validator_gates/{WP_ID}.json`
- ROLE_MAILBOX_EXPORT_DIR: `.GOV/ROLE_MAILBOX/`

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

- CARGO_TARGET_DIR (external): `../Handshake Artifacts/handshake-cargo-target`
- NODE_PACKAGE_MANAGER: `pnpm` (for `app/`)

## 7) Product-owned runtime governance state (default)

- RUNTIME_GOV_STATE_DIR: `.handshake/gov/` (configurable; runtime governance state only)
