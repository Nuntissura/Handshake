# Handshake

Local-first desktop app that combines workspaces, documents, canvases, governed workflows, and later local AI/model-assisted runtime features.

This repo contains:
- the desktop shell (`app/`, Tauri + React + TypeScript)
- the Rust backend (`src/backend/handshake_core/`)
- the governance/workflow system (`.GOV/`)

---

## Vision / Research

- [Project Vision & Technical Synthesis](./.GOV/reference/research_and_papers/HANDSHAKE_VISION_SYNTHESIS.md)

This is optional background reading. It is not the authoritative governance entrypoint.

---

## Start Here

- Product/governance entrypoint: [`.GOV/roles_shared/docs/START_HERE.md`](./.GOV/roles_shared/docs/START_HERE.md)
- Repo law and placement rules: [`.GOV/codex/Handshake_Codex_v1.4.md`](./Handshake%20Codex%20v1.4.md)
- Current product spec pointer: [`.GOV/spec/SPEC_CURRENT.md`](./.GOV/spec/SPEC_CURRENT.md)

---

## Repository Layout

- **Desktop app (Tauri + React + TS)**  
  `app/`

- **Backend crate (Rust, health + API logic)**  
  `src/backend/handshake_core/`

- **Governance / workflow / tasking**  
  `.GOV/`

- **Shared product assets**  
  `assets/`

- **Top-level tests / placeholders / harnesses**  
  `tests/`

---

## Development Commands

You can use the `just` shortcuts when available, or call the underlying commands directly.

```bash
# Start the desktop app in dev mode
just dev
```

Equivalent direct command:

```bash
pnpm -C app tauri dev
```

---

## Testing

```bash
cargo test --manifest-path src/backend/handshake_core/Cargo.toml
```

```bash
pnpm -C app test
```

```bash
just validate
```

---

## Linting

```bash
pnpm -C app run lint
```

```bash
just lint
```

---

## Governance Checks

For governance-only changes:

```bash
just gov-check
```

For the human/model governance entrypoint:

```bash
Get-Content .GOV/roles_shared/docs/START_HERE.md
```

---

## Runtime / Artifact Paths

- External build/test artifacts live outside the repo under:
  - `../Handshake Artifacts/`
- Current default target for future product runtime state is:
  - `../Handshake Runtime/`
- Existing repo-root runtime paths such as `data/` and `.handshake/` are transitional legacy surfaces during the current early Phase 1 state.

---

## Current Status

- **Phase 0** (diagnostic vertical slice) is complete and committed.
- The app and governance system are both under active restructuring and early Phase 1 development.
- Runtime/governance/product boundaries are being tightened, but some legacy/transitional surfaces still exist.

Use this README for a quick repo overview only. For active workflow/governance law, use the Codex and role protocols under `.GOV/roles/`.
