# ARCHITECTURE

| Module/Area | Responsibility | Entry files/dirs | Allowed dependencies | Where to add features |
| --- | --- | --- | --- | --- |
| .claude/ (Claude Code instructions) | Local AI prompt/instruction storage for Claude Code | `.claude/` | None | Do not add features; instructions only |
| Frontend shell (Tauri + React) | Desktop window, UI components, invokes backend | `app/src/main.tsx`, `app/src/`, `app/src-tauri/src/lib.rs` | Uses Tauri APIs, frontend packages, shared TS types when they land; may call backend via IPC/HTTP; avoid direct DB/filesystem writes except via Tauri | New UI flows/components in `app/src`; new Tauri commands/wiring in `app/src-tauri/src/lib.rs` |
| Backend core (Rust) | API + orchestration, data access, logging | `src/backend/handshake_core/src/main.rs`, `src/backend/handshake_core/src/api/*.rs`, `models.rs`, `logging.rs` | Rust crates; SurrealDB/EventLedger access only through the storage boundary and official SurrealDB Rust SDK; expose commands/endpoints for frontend; do not depend on frontend code; legacy SQL paths are WP-KERNEL-017 removal inventory, not extension points | Add endpoints in `src/backend/handshake_core/src/api/`; data models in `models.rs`; logging via `logging.rs`; add database operations through `storage::` only |
| Data + SurrealKit rollouts | Typed `SCHEMAFULL` schema, bootstrap records, indexes, permissions, rollout/cutover/rollback | SurrealDB storage boundary and SurrealKit rollout surfaces declared by the `SPEC_CURRENT`-resolved Master Spec; legacy `src/backend/handshake_core/migrations/` is removal/translation inventory | Touched by backend storage boundary only; no ad-hoc schema drift; no SQLite/PostgreSQL connectivity, import, reconciliation, fallback, fixture, cache, or proof path | Add/modify versioned SurrealKit rollouts and typed SurrealQL definitions; initialize WP-scoped SurrealDB namespace/database proof state; runtime logs land in `data/logs/` |
| Shared contracts | Cross-stack types and schemas | `src/shared/` | Intended for dual Rust/TS types; TBD (HSK-1002): define actual shared types | Place shared DTOs/schemas here when ready; update both stacks to consume them |
| Packet closure / acceptance monitor | Executable packet closure contract: clause rows, acceptance rows, spec-debt reflection, semantic proof and shared-surface monitoring | `.GOV/templates/TASK_PACKET_TEMPLATE.md` | Reads/writes packet markdown through packet creation/normalization only; validators consume rows as authority for PASS legality | Add new acceptance row fields or closure monitor statuses here; new packets must emit `PACKET_ACCEPTANCE_MATRIX` instead of relying on prose-only acceptance |
| Role startup briefs | Memory-Manager-curated operational anti-repeat cards printed at role startup; compiles repeated procedural failures into role/pathing/toolcalling guidance without becoming protocol law | `.GOV/roles_shared/docs/STARTUP_BRIEF_SCHEMA.md`, `.GOV/roles_shared/docs/SHARED_STARTUP_BRIEF.md`, `.GOV/roles/*/docs/*_STARTUP_BRIEF.md` | Reads brief markdown only; Memory Manager may edit startup brief files, but protocols/Codex/packets remain higher authority; Orchestrator or Classic Orchestrator reviews broader Memory Manager proposals before governance changes are made | Add new role/action anti-repeat cards under the owning role's `docs/*_STARTUP_BRIEF.md` or shared cards under `SHARED_STARTUP_BRIEF.md`; propose or implement tooling repair through the active coordinator when the repeated failure is mechanical enough to enforce |

Note: Frontend and Tauri shell live under `app/` and `app/src-tauri/` (codex deviation from `/src/frontend` convention). Backend crate lives under `src/backend/handshake_core/`.

Feature flags/toggles: If introducing flags, document the flag name and location here and in relevant modules.

## Governance Kernel Path Resolution [CX-212B/C]

Removed 2026-09-23: the command surface was deleted with the governance harness.

The **governance kernel worktree** is a dedicated worktree holding the canonical `.GOV/` copy; role worktrees read governance from that single canonical source. No cherry-picking or propagation is needed.

## Raw / Derived / Display (RDD)
- Raw: SurrealDB/EventLedger is the only forward authority. Existing SQLite/PostgreSQL-backed product content and SQL migrations are legacy WP-KERNEL-017 removal/translation inventory; new work must not extend or prove against them.
- Derived: TBD (HSK-1003) - no concrete derived pipeline is implemented yet; track when indexing/embeddings land.
- Display: UI rendering in `app/src/` (DocumentView/CanvasView) builds display state from backend responses; no persisted display layer yet.

## Inter-Role Wire Format [CX-130]

Communication between governed roles is structured event traffic, not free-form prose. The wire is the receipt and notification schema family:

| Surface | Schema family | Authority |
| --- | --- | --- |
| Receipts | `WP_RECEIPT.schema.json` | role-to-role events (`CODER_INTENT`, `CODER_HANDOFF`, validator review responses, verdicts) |
| Notifications | `WP_NOTIFICATION.schema.json` | governance-routing wakes and alerts |

Projection artifacts — WP packets (`packet.md`), Workflow Dossier (`templates/WORKFLOW_DOSSIER_TEMPLATE.md`), validator reports, post-mortems — are **projections rendered from receipt and notification truth**. They are not the wire between roles. Roles MUST NOT author them as a substitute for emitting structured receipts; the projection layer materializes them for role/tool review by default, and for operator-readable review only when explicitly requested or required by a report/projection contract.

This separation is load-bearing: model-authored prose between roles is the documented dominant token-cost driver. Typed events eliminate the malformation/repair loops, the cache invalidation on doc revisions, and the read-cost of paragraphs the receiving model cannot reliably parse. Role/tool consumption stays typed; operator readability is available through explicit or contract-bound projections. See Codex `[CX-130]` and `[CX-130A]`.
