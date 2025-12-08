# Handshake

Local-first desktop app that combines workspaces, documents, canvases, and (later) local AI models.
This repo contains both the Tauri desktop shell (React + TypeScript) and the Rust backend.

---

## Repository layout

- **Desktop app (Tauri + React + TS)**  
  `app/`

- **Backend crate (Rust, health + API logic)**  
  `src/backend/handshake_core/`

There are additional crates/modules under `src/backend/` as the project grows.

---

## Development commands (from repo root)

You can use the `just` shortcuts when available, or call the underlying commands directly.

```bash
# Start the desktop app in dev mode (recommended)
pnpm -C app tauri dev
```

If a `justfile` is present and configured, you can also run:

```bash
just dev
```

---

## Testing

### Backend (Rust)

Runs tests for the main backend crate (including health-response tests):

```bash
cargo test --manifest-path src/backend/handshake_core/Cargo.toml
```

### Frontend (React + TypeScript)

Uses Vitest + React Testing Library:

```bash
pnpm -C app test
```

If a `justfile` exposes a combined test alias, you can optionally run:

```bash
just test
```

---

## Linting

Frontend lint (from repo root):

```bash
pnpm -C app run lint
```

If a `justfile` provides lint helpers, they can be used as well, for example:

```bash
just lint
```

---

## Phase 0 / Phase 0.5 status (high level)

- **Phase 0** (diagnostic vertical slice) is complete and committed.
- The editor vertical slice uses a Tiptap-based document editor with:
  - Sticky toolbar and scrollable worksurface.
  - Plain-text persistence (formatting is not yet stored).
- A minimal automated test scaffold exists:
  - Backend health-response unit tests.
  - Frontend shell render test (App header / coordinator status).

Use this README as the reference for how to run, test, and iterate on the current codebase.
