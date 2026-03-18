# ARCHITECTURE

| Module/Area | Responsibility | Entry files/dirs | Allowed dependencies | Where to add features |
| --- | --- | --- | --- | --- |
| .claude/ (Claude Code instructions) | Local AI prompt/instruction storage for Claude Code | `.claude/` | None | Do not add features; instructions only |
| Frontend shell (Tauri + React) | Desktop window, UI components, invokes backend | `app/src/main.tsx`, `app/src/`, `app/src-tauri/src/lib.rs` | Uses Tauri APIs, frontend packages, shared TS types when they land; may call backend via IPC/HTTP; avoid direct DB/filesystem writes except via Tauri | New UI flows/components in `app/src`; new Tauri commands/wiring in `app/src-tauri/src/lib.rs` |
| Backend core (Rust) | API + orchestration, data access, logging | `src/backend/handshake_core/src/main.rs`, `src/backend/handshake_core/src/api/*.rs`, `models.rs`, `logging.rs` | Rust crates, SQLite via migrations; expose commands/endpoints for frontend; do not depend on frontend code | Add endpoints in `src/backend/handshake_core/src/api/`; data models in `models.rs`; logging via `logging.rs` |
| Data + migrations | Schema, seeds, storage layout | `src/backend/handshake_core/migrations/`, `data/` runtime artifacts | Touched by backend only; migrations structured for SQLite; no ad-hoc schema drift | Add/modify migrations under `migrations/`; runtime logs land in `data/logs/` |
| Shared contracts | Cross-stack types and schemas | `src/shared/` | Intended for dual Rust/TS types; TBD (HSK-1002): define actual shared types | Place shared DTOs/schemas here when ready; update both stacks to consume them |
| Tooling / governance runtime | Developer ergonomics, workflow automation, governance enforcement | `justfile`, `.GOV/roles/*/{scripts,checks}/`, `.GOV/roles_shared/{scripts,checks}/`, `.GOV/roles_shared/scripts/hooks/` | Shell/CLI dependencies only; do not bake product business logic here | Add repeatable tasks to `justfile`; place role-owned tooling under the role bundle and shared tooling under `roles_shared` |

Note: Frontend and Tauri shell live under `app/` and `app/src-tauri/` (codex deviation from `/src/frontend` convention). Backend crate lives under `src/backend/handshake_core/`.

Feature flags/toggles: If introducing flags, document the flag name and location here and in relevant modules.

## Governance Kernel Path Resolution [CX-212B/C]

All governance scripts and justfile recipes resolve the governance root through the `HANDSHAKE_GOV_ROOT` environment variable instead of hardcoding `.GOV/`.

| Surface | Resolution mechanism | Fallback |
| --- | --- | --- |
| Node.js scripts | `GOV_ROOT_REPO_REL` from `runtime-paths.mjs` | `.GOV/` (relative to repo root) |
| Justfile recipes | `{{GOV_ROOT}}` via `env_var_or_default('HANDSHAKE_GOV_ROOT', '.GOV')` | `.GOV/` |

This enables a **governance kernel worktree**: a dedicated worktree holding the canonical `.GOV/` copy. Role worktrees set `HANDSHAKE_GOV_ROOT` to point at the kernel, so all governance execution (checks, scripts, gates) runs from the single canonical source. No cherry-picking or propagation is needed.

## Raw / Derived / Display (RDD)
- Raw: SQLite-backed content is persisted by the backend (`src/backend/handshake_core/migrations/` and API handlers in `src/backend/handshake_core/src/api/`).
- Derived: TBD (HSK-1003) - no concrete derived pipeline is implemented yet; track when indexing/embeddings land.
- Display: UI rendering in `app/src/` (DocumentView/CanvasView) builds display state from backend responses; no persisted display layer yet.
