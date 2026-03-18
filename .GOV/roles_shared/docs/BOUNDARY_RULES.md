# REPO_BOUNDARY_RULES

This file defines the **hard boundary** between:
- **Repo governance workspace** (for humans/LLMs and enforcement tooling), and
- **Product runtime** (Handshake binaries/libraries/tests).

These rules exist to prevent the product from accidentally depending on the evolving governance workspace.

Authoritative LAW: `Handshake Codex v1.4.md` ([CX-211], [CX-212]).

---

## 1) `.GOV/` = Governance Workspace (Authoritative for workflow)

`.GOV/` is the canonical location for:
- role protocols and active governance tooling
- gate definitions + validator/orchestrator state
- governance implementation (`.GOV/roles/**`, `.GOV/roles_shared/**`)
- task packets + refinements + templates
- governance logs and operator materials
- non-authoritative historical/reference material under `.GOV/reference/`

**Hard rule:** Product runtime MUST NOT read from or write to `.GOV/` under any circumstances.

---

## 2) `docs/` = Product Compatibility Bundle (Temporary)

`docs/` exists to keep the **current** Handshake product implementation working, because it historically hardcoded repo-relative paths (e.g., `docs/TASK_BOARD.md`, `docs/OSS_REGISTER.md`).

**Hard rule:** Governance workflow/tooling MUST NOT treat `docs/` as authoritative governance state.

**Status:** Temporary compatibility only, until `WP-1-Product-Governance-Snapshot-v1` (name retained) removes runtime dependency on repo `docs/`.

---

## 3) `.handshake/gov/` = Product-Owned Runtime Governance State (Default)

Handshake runtime governance state MUST live in product-owned storage, not in repo governance folders.

- Default location: `.handshake/gov/`
- Configurable: YES (must remain deterministic and explicit)
- Scope: **runtime governance state only** (not a mirror of repo governance)

---

## 4) Governance Kernel Worktree [CX-212B/C]

The governance root path is resolved at runtime via the `HANDSHAKE_GOV_ROOT` environment variable:
- **Scripts:** Use `GOV_ROOT_REPO_REL` from `.GOV/roles_shared/scripts/lib/runtime-paths.mjs` (falls back to `.GOV/` when env var is unset).
- **Justfile:** Uses `GOV_ROOT := env_var_or_default('HANDSHAKE_GOV_ROOT', '.GOV')`.
- **Purpose:** Enables a shared governance kernel worktree — one canonical `.GOV/` copy that all role worktrees execute from, eliminating cherry-pick ancestry contamination.
- **Kernel contents:** Scripts, checks, protocols, schemas, templates, validator gate configs, task board, build order, spec, traceability registry.
- **NOT in kernel:** WP communications, runtime session state, audits — these remain WP-local under the external repo-governance runtime root.
- **Write access:** The managing orchestrator (model) reads from the kernel but MUST NOT write to it. Governance edits require a separate model session.

All `.GOV/` path references in codex, protocols, and boundary docs refer to the logical governance root, which resolves to the kernel worktree path when `HANDSHAKE_GOV_ROOT` is set.

---

## 5) Enforcement

The repo MUST enforce (via CI/gates):
- No product code references to `/.GOV/` (strings, paths, or file I/O).

Future (post-remediation):
- No runtime-critical reads of `docs/**` (product should use embedded resources + product-owned state).
