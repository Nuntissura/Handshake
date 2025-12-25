# OSS REGISTER

**Authoritative Open Source Software Manifest**  
**Status:** ACTIVE  
**Updated:** 2025-12-25 (Rebuild from current manifests; prior copy was deleted)

> Scope: Captures all dependencies and dev/build tools declared in `Cargo.toml` (backend + Tauri) and `package.json` (frontend). Copyleft guard remains default-deny (GPL/AGPL only via `external_process`, none present today).

## Backend – `src/backend/handshake_core/Cargo.toml`

| Component | License | Scope | Purpose |
| --- | --- | --- | --- |
| axum | MIT | Runtime | HTTP server (REST API) |
| serde | MIT/Apache-2.0 | Runtime | Serialization/Deserialization |
| serde_json | MIT/Apache-2.0 | Runtime | JSON handling |
| tokio | MIT | Runtime | Async runtime (macros, process, time) |
| tower-http | MIT | Runtime | HTTP middleware (CORS) |
| sqlx | MIT | Runtime | DB driver (SQLite, migrations, chrono) |
| uuid | MIT/Apache-2.0 | Runtime | UUID generation/serde |
| chrono | MIT/Apache-2.0 | Runtime | Time handling (serde/clock) |
| tracing | MIT | Runtime | Structured logging |
| tracing-subscriber | MIT | Runtime | Logging sinks/filters (fmt/json) |
| tracing-appender | MIT | Runtime | Log file appender |
| thiserror | MIT/Apache-2.0 | Runtime | Error derivations |
| duckdb | MIT | Runtime | Analytics / Flight Recorder (bundled) |
| reqwest | MIT/Apache-2.0 | Runtime | HTTP client (Ollama integration) |
| async-trait | MIT/Apache-2.0 | Runtime | Async trait support |
| once_cell | MIT/Apache-2.0 | Runtime | Lazy init (metric sinks, registries) |

## Desktop Shell – `app/src-tauri/Cargo.toml`

| Component | License | Scope | Purpose |
| --- | --- | --- | --- |
| tauri | MIT/Apache-2.0 | Runtime | Desktop shell / IPC bridge |
| tauri-plugin-opener | MIT/Apache-2.0 | Runtime | Safe “open” integration |
| serde | MIT/Apache-2.0 | Runtime | Serialization/Deserialization |
| serde_json | MIT/Apache-2.0 | Runtime | JSON handling |
| tauri-build | MIT/Apache-2.0 | Build | Tauri build script support |

## Frontend Runtime – `app/package.json` dependencies

| Component | License | Scope | Purpose |
| --- | --- | --- | --- |
| @excalidraw/excalidraw | MIT | Runtime | Canvas / whiteboard |
| @tauri-apps/api | MIT/Apache-2.0 | Runtime | Tauri IPC and shell APIs |
| @tauri-apps/plugin-opener | MIT/Apache-2.0 | Runtime | Link/file opener bridge |
| @tiptap/core | MIT | Runtime | Rich-text core |
| @tiptap/extension-collaboration | MIT | Runtime | CRDT-backed editing |
| @tiptap/react | MIT | Runtime | React bindings for TipTap |
| @tiptap/starter-kit | MIT | Runtime | Default editor nodes/marks |
| react | MIT | Runtime | UI framework |
| react-dom | MIT | Runtime | React DOM renderer |
| yjs | MIT | Runtime | CRDT collaboration |

## Frontend Tooling & Tests – `app/package.json` devDependencies

| Component | License | Scope | Purpose |
| --- | --- | --- | --- |
| @eslint/js | MIT | Dev | ESLint config |
| @tauri-apps/cli | MIT/Apache-2.0 | Dev | Tauri CLI/build |
| @testing-library/jest-dom | MIT | Test | Jest DOM matchers |
| @testing-library/react | MIT | Test | React testing utilities |
| @types/jsdom | MIT | Dev | TypeScript types |
| @types/react | MIT | Dev | TypeScript types |
| @types/react-dom | MIT | Dev | TypeScript types |
| @typescript-eslint/eslint-plugin | MIT | Dev | ESLint rules for TS |
| @typescript-eslint/parser | MIT | Dev | ESLint parser for TS |
| @vitejs/plugin-react | MIT | Dev | Vite React plugin |
| dependency-cruiser | MIT | Dev | Dependency graph linting |
| eslint | MIT | Dev | Lint runner |
| eslint-plugin-react-hooks | MIT | Dev | Hooks lint rules |
| globals | MIT | Dev | Global definitions for ESLint |
| jsdom | MIT | Test | DOM emulation for tests |
| typescript | Apache-2.0 | Dev | TypeScript compiler |
| vite | MIT | Dev | Bundler |
| vitest | MIT | Test | Test runner |

## Governance & Enforcement

1) **Copyleft isolation:** GPL/AGPL components are disallowed unless run as `external_process` with no static linking (none present in current manifests).  
2) **Coverage rule:** Every dependency declared in `Cargo.toml`/`Cargo.lock`/`package.json`/`package-lock equivalents` must appear in this register; update this file whenever dependencies change.  
3) **Security gate:** Supply-chain checks (e.g., `just validate`, `cargo deny`, npm audit equivalents) must be remediated within 48 hours or blocked at merge.  
4) **Evidence:** Register updates should cite the manifest/lock source in PR descriptions to keep provenance auditable.
