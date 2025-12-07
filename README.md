# Handshake

## Layout
- Desktop app (Tauri + React + TS): `app/`
- Backend crate (Rust): `src/backend/handshake_core/`

## Commands (from repo root)
- Dev app: `just dev`
- Lint (frontend + backend): `just lint`
- Test backend: `just test`

## Frontend package manager
- Frontend uses **pnpm** (see `app/pnpm-lock.yaml`).
- From `app/`: `pnpm run tauri dev`, `pnpm run lint`.
