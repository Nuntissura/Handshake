# Handshake Workflow Paths
Machine-specific absolute paths for the Handshake project on this host.

---

## 1. Core Roots

- **Index root (all LLM projects)**
  - `D:\Projects\LLM projects`

- **Handshake repo root**
  - `D:\Projects\LLM projects\Handshake`

---

## 2. Governance / Spec Documents

- **Master Spec (canonical)**
  - `D:\Projects\LLM projects\Handshake\Handshake_Master_Spec_v02.12.md`

- **Codex v0.5**
  - `D:\Projects\LLM projects\Handshake\Handshake Codex v0.5.md`

- **Handshake logger**
  - `D:\Projects\LLM projects\Handshake\Handshake_logger_v3.1.md`

- **Workflow notes**
  - `D:\Projects\LLM projects\Handshake\Handshake workflow.md`

- **Roadmap section file (outside repo root)**
  - `D:\Projects\LLM projects\Handshake_Section_7.6_Development_Roadmap_v0.2.md`

---

## 3. App / Runtime Paths

- **Tauri app root (desktop shell MVP)**
  - `D:\Projects\LLM projects\Handshake\app`

- **Frontend (React + Vite)**
  - `D:\Projects\LLM projects\Handshake\app\src`

- **Backend (Rust + Tauri)**
  - `D:\Projects\LLM projects\Handshake\app\src-tauri`

---

## 4. Repo Folders (Existing)

- **Legacy `src` folder (meaning TBD)**
  - `D:\Projects\LLM projects\Handshake\src`

- **Docs local folder**
  - `D:\Projects\LLM projects\Handshake\docs_local`

- **Archive folder**
  - `D:\Projects\LLM projects\Handshake\archive`

---

## 5. Indices

- **Local docs index**
  - `D:\Projects\LLM projects\Handshake\docs_local\DOC_INDEX.txt`

- **Note on index JSON files**
  - Index JSON files for the Python indexer live under the Windows filesystem rooted at:  
    `D:\Projects\LLM projects\...`  
  - Assistants MUST NOT invent new index paths; the user will provide actual index JSON filenames when needed.

---

## 6. Behaviour Rules for Assistants (Paths)

1. Assistants MUST treat the paths in this document as the single source of truth for absolute locations on this host.
2. Assistants MUST NOT invent or guess new absolute paths under `D:\Projects\LLM projects\...`.
3. When a task requires a file not listed here, assistants MUST ask the user for the path or for the latest index output.
4. All code/CLI examples MUST use these exact paths when referring to the items above.
