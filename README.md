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
- Repo law and placement rules: [`Handshake Codex v1.4.md`](./Handshake%20Codex%20v1.4.md)
- Current product spec pointer: [`.GOV/roles_shared/records/SPEC_CURRENT.md`](./.GOV/roles_shared/records/SPEC_CURRENT.md)

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
  - `../Handshake_Artifacts/`
- Governance/runtime coordination state lives outside product worktrees under:
  - `../gov_runtime/`
- Existing repo-root runtime paths such as `data/` and `.handshake/` are transitional legacy surfaces during the current early Phase 1 state.

---

## Current Status

- **Phase 0** (diagnostic vertical slice) is complete and committed.
- The app and governance system are both under active restructuring and early Phase 1 development.
- Runtime/governance/product boundaries are being tightened, but some legacy/transitional surfaces still exist.

---

## KB003 Sandbox / Validation / Promotion (no-context model manual)

KB003 is the kernel sandbox + validation + promotion pipeline: every sandbox run produces typed evidence, gets validated against a deterministic descriptor batch, then either promotes through `PromotionGate` or rejects with a typed reason. No step is ever "trust the chat history".

**Where the modules live** (`src/backend/handshake_core/src/kernel/`):

- `sandbox/` — adapter trait, default-deny policy, fs/network/exec guards, denial taxonomy, replay-projection contract (`run`, `policy`, `policy_default_deny`, `fs_guard`, `network_gate`, `exec_allowlist`, `denial`, `replay_projection`, `workspace_materializer`, `hard_isolation*`, `no_sqlite_tripwire`).
- `validation/` — descriptor runner, status taxonomy, report assembler, redaction report (`run`, `descriptor`, `status`, `report`, `redaction_report`).
- `kb003_promotion/` — `PromotionGate::evaluate` + `PromotionDecisionV1` + `PromotionReceiptV1` (`gate`, `decision`, `receipt`, `artifact_bundle`, `event_emission`, `dcc_promotion_overlay`).
- `mte_*` — per-microtask layer: resource caps, blocked taxonomy, retry budget, idempotency enforcement, closeout bundle, validation-report projection, authority mutation boundary.
- `dcc_kb003_*` — operator-facing DCC projections (rollup, blocked reasons, lane wake, promotion control state, manual hints, run detail, sandbox run list, aggregate summary).

**How a no-context model inspects sandbox state** — read the DCC projection first, never raw logs:

1. `DccKb003RollupV1::new(...)` is the single top-level operator view; it composes the sandbox projection, blocked-reason overlay, lane-wake timeline, promotion control state, manual hints, and optional WP-scope aggregate.
2. `DccSandboxProjectionV1::summary_line()` returns a one-line status; `is_self_describing()` confirms the projection carries the evidence its outcome implies (denial / validation / promotion summaries).
3. For promotion specifically, look up `PromotionReceiptV1` via `Kb003Storage::load_replay_bag(run_id, policy_version_id)` — that bag is the only durable source a replay is allowed to read.

**Idempotency** is two-layer: MTE chain (`mte_idempotency_enforcement`) collapses per-microtask retries; `PromotionGate` keys receipts on `(idempotency_key, payload_hash)` via `Kb003Storage::insert_promotion_receipt`. Same key + same payload returns the original receipt id; same key + different payload surfaces as `PromotionRejectionReason::DuplicateIdempotencyKey`.

**Redaction-aware export** — `RedactionReport::partition_default_policy(members)` splits a candidate export list by `Kb003ArtifactClass.exportable_by_default`. Screenshots and redaction notes are non-exportable by default; the report records what was withheld and why.

**Hard rules**:

- No SQLite authority for KB003 writes (CX-503R). `kernel::sandbox::no_sqlite_tripwire::guard_authority_write(...)` fails closed on every mode except `PostgresPrimary`.
- All artifact retention roots live under `handshake-product/` (CX-212E); absolute resolution is the storage layer's job.
- The WP Validator never self-validates — Integration Validator runs the whole-WP review batch after coder/WP-validator handoff.

---

Use this README for a quick repo overview only. For active workflow/governance law, use the Codex and role protocols under `.GOV/roles/`.
