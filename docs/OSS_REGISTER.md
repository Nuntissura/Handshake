# OSS REGISTER

**Authoritative Open Source Software Manifest**
**Status:** ACTIVE
**Updated:** 2025-12-25 (Reconstructed after Stabilization)

## 1. Backend Dependencies (Rust)

| Component | License | Mode | Purpose |
| :--- | :--- | :--- | :--- |
| **async-trait** | MIT/Apache-2.0 | Integrated | Async trait support |
| **axum** | MIT | Integrated | Web framework (REST API) |
| **chrono** | MIT/Apache-2.0 | Integrated | Date and time handling |
| **duckdb** | MIT | Integrated | Analytics / Flight Recorder |
| **once_cell** | MIT/Apache-2.0 | Integrated | Lazy initialization |
| **reqwest** | MIT/Apache-2.0 | Integrated | HTTP client (Ollama integration) |
| **serde** | MIT/Apache-2.0 | Integrated | Serialization/Deserialization |
| **sqlx** | MIT/Apache-2.0 | Integrated | Database driver (SQLite) |
| **tokio** | MIT | Integrated | Async runtime |
| **tower-http** | MIT | Integrated | CORS and HTTP middleware |
| **tracing** | MIT | Integrated | Logging and observability |
| **uuid** | MIT/Apache-2.0 | Integrated | UUID generation |

## 2. Frontend Dependencies (NPM)

| Component | License | Mode | Purpose |
| :--- | :--- | :--- | :--- |
| **@excalidraw/excalidraw** | MIT | Integrated | Interactive Canvas |
| **@tauri-apps/api** | MIT/Apache-2.0 | Integrated | Tauri IPC and shell |
| **@tiptap/core** | MIT | Integrated | Rich-text editor foundation |
| **react** | MIT | Integrated | UI Framework |
| **yjs** | MIT | Integrated | CRDT / Real-time collaboration |

## 3. Governance Policies

1.  **Copyleft Isolation:** Components under GPL or AGPL licenses MUST NOT be statically linked. They must be executed as `external_process` only.
2.  **Audit Requirement:** Every dependency in the codebase MUST have a corresponding entry in this register.
3.  **Security Gate:** Unmaintained or vulnerable packages flagged by `just validate` must be remediated within 48 hours.
