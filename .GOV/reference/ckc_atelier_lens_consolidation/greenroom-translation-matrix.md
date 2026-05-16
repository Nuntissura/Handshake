---
file_id: ckc-greenroom-translation-matrix
file_kind: greenroom_translation_matrix
updated_at: 2026-05-16
status: reference_only_not_execution_authority
---

<topic id="translation-stance" status="active" version="v1" summary="Translation stance" updated_at="2026-05-16">

# Translation Stance

CKC is not ported wholesale. CKC is translated into Handshake-native modules.

Preserve:

- Domain behavior.
- Source requirements.
- Tests and fixtures.
- Operator workflows.
- Model-facing automation ideas.
- Media/character/ComfyUI/PoseKit provenance goals.

Adapt:

- JS backend services into Rust services.
- Electron IPC into Tauri commands and Rust API boundaries.
- CKC file/folder assumptions into Handshake artifact roots.
- CKC tests into Rust/Tauri/Postgres parity tests.
- CKC automation manual into Handshake model manual/action catalog.

Reject for runtime:

- SQLite in any form. Handshake must not accept SQLite in runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, or product paths.
- Electron main/preload as authority.
- Localhost intake as authority.
- Product outputs under `.GOV`.
- CKC product namespace in Handshake runtime.

</topic>

<topic id="module-boundaries" status="active" version="v1" summary="Handshake-native module boundaries" updated_at="2026-05-16">

# Proposed Handshake Module Boundaries

- `atelier_core`: character IDs, profile metadata, public/internal ID split, shared error types.
- `atelier_sheet`: template parser, sheet parser, validation, protected fields, versioning, diff/apply.
- `atelier_media`: media assets, hashes, source refs, review state, tags, ratings, favorites, missing-media diagnostics.
- `atelier_intake`: batch creation, item classification, accept/reject/pending state machine, linked/loose imports.
- `atelier_collections`: collections, contact-sheet manifests, moodboard refs, collection tags/notes.
- `atelier_sidecars`: OpenPose/sidecar refs, archive/restore, sidecar visibility projection.
- `atelier_posekit`: rig metadata, OpenPose schema, identity profiles, calibration debt markers.
- `atelier_comfy`: workflow spec registry, recipe receipt, run receipt, artifact proposal, replay lineage.
- `atelier_search`: search projection contract, tags, links/backlinks, similarity/palette hooks.
- `atelier_exports`: export manifest, backup manifest, share packs, no-space names, provenance.
- `atelier_automation`: action catalog, model manual, leases, command receipts, screenshot/debug hooks.
- `kernel_event_bridge`: EventLedger event builders for Atelier/Lens actions.

</topic>

<topic id="conflict-matrix" status="active" version="v1" summary="Conflicts and required resolutions" updated_at="2026-05-16">

# Conflict Matrix

| Conflict | CKC Assumption | Handshake Resolution |
|---|---|---|
| Storage | SQLite, FTS5, runtime DDL, SQL translator, `db/codex.db` fixtures | PostgreSQL/EventLedger authority only. Do not bring SQLite into Handshake in any form, including runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, or product paths. |
| App shell | Electron main/preload, BrowserWindow, IPC bridge | Tauri/Rust command boundary and React projection. |
| Intake | Localhost CKC intake endpoint as runtime authority | Typed Handshake endpoint or artifact proposal path with EventLedger lineage. |
| Product namespace | `CKC`, `CastKit Codex`, `CKC_main`, `CKC_GOV`, `CastKitCodexBridge` | Handshake/Atelier/Lens namespace and portable no-space artifact names. |
| Product artifacts | CKC outputs under `CKC_GOV/targets` | Product runtime/artifact roots, not `.GOV`. |
| Paths | D-drive sample paths and old OpenRepose paths | Historical evidence only; runtime paths are repo-relative, artifact-root-relative, or operator-configured. |
| Search | SQLite FTS5 implementation details | Preserve search behavior; use Handshake search/index architecture. |
| Automation authority | Process-local sessions/leases | Postgres/EventLedger-backed leases and receipts. |
| UI scope | CKC feature-rich React/Electron views | Later React projections after kernel contracts; no GUI parity in Kernel WP. |
| Sidecars | File-adjacent sidecar behavior | Typed artifact rows/refs and projection controls. |

</topic>

<topic id="source-test-translation" status="active" version="v1" summary="Source tests to preserve as parity evidence" updated_at="2026-05-16">

# Source Test Translation

Preserve CKC tests as behavior evidence, not runtime proof.

Translate into Handshake parity tests for:

- Sheet parser and protected field behavior.
- Template parser and descriptor validation.
- No silent rewrite / byte preservation.
- Character public/internal ID split.
- Media provenance and missing-media diagnostics.
- Intake idempotency and state transitions.
- Sidecar hiding and OpenPose JSON shape.
- PoseKit hand/body/face/head-pose schema.
- ComfyUI bridge payload and workflow receipt shape.
- Automation command map/manual consistency.
- Backup/export manifest checksum behavior.
- Tags/search/similarity projection behavior.

</topic>

<topic id="activation-concerns" status="active" version="v1" summary="Activation concerns and mitigations" updated_at="2026-05-16">

# Activation Concerns

- Concern: three massive WPs can overwhelm validation.
  - Mitigation: each WP must use no-context microtasks and source-path-backed acceptance rows.

- Concern: Greenroom becomes another governance detour.
  - Mitigation: Greenroom output is bounded to registers, matrices, fixtures, and packet handoff; no product code.

- Concern: Kernel WP accidentally becomes GUI work.
  - Mitigation: Kernel WP excludes full GUI and only exposes Tauri/API projections needed by later UI.

- Concern: old stubs get silently overwritten by CKC priority.
  - Mitigation: every prior stub is classified as source material, baseline, inherited requirement, separate dependency, or later media-intelligence path.

- Concern: CKC runtime shortcuts leak into Handshake.
  - Mitigation: SQLite is an absolute no-go before coding; Electron and localhost intake are reject-for-runtime.

</topic>
