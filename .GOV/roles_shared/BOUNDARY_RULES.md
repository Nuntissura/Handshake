# REPO_BOUNDARY_RULES

This file defines the **hard boundary** between:
- **Repo governance workspace** (for humans/LLMs and enforcement tooling), and
- **Product runtime** (Handshake binaries/libraries/tests).

These rules exist to prevent the product from accidentally depending on the evolving governance workspace.

Authoritative LAW: `Handshake Codex v1.4.md` ([CX-211], [CX-212]).

---

## 1) `.GOV/` = Governance Workspace (Authoritative for workflow)

`.GOV/` is the canonical location for:
- role protocols + rubrics
- gate definitions + validator/orchestrator state
- governance scripts (`.GOV/scripts/**`)
- task packets + refinements + templates
- governance logs and operator materials

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

## 4) Enforcement

The repo MUST enforce (via CI/gates):
- No product code references to `/.GOV/` (strings, paths, or file I/O).

Future (post-remediation):
- No runtime-critical reads of `docs/**` (product should use embedded resources + product-owned state).
