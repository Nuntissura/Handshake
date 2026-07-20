---
file_id: stage-mvp-source-preservation-register
file_kind: reference-source-disposition-register
updated_at: "2026-07-19"
status: source-disposition-map-ready-for-review
wp_id: WP-1-Handshake-Stage-MVP-v1
---

<topic id="stage-mvp-source-preservation-register" status="source-disposition-ready-for-review" version="v0.3" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Stage source-disposition register

This register records how older Stage source material was considered while building the current plan. It is a non-authoritative lineage map: no source stub is superseded, archived, or edited by this file.

The operator has locked that the current Stage direction supersedes all older Stage-specific product direction and implementation, including built prototypes, adapters, connectors, schemas, routes, panes, and mockups. Therefore `STAGE-PRES-*` IDs preserve provenance only. A row survives in the future contract only when its behavior has been independently restated by a current `STAGE-REQ-*` requirement or later operator decision. No row creates a legacy compatibility, implementation reuse, data migration, or UX preservation obligation. Shared non-Stage Handshake authorities remain external dependencies, and real operator data must receive an explicit non-loss disposition.

## Fold boundary

| Source | Disposition | Rule |
|---|---|---|
| `WP-1-Handshake-Stage-MVP-v1` | Retain only as the destination umbrella ID | Regenerate its content from the current Stage corpus. Its former product shape and any implementation behind it are superseded. |
| `WP-1-Stage-Media-Artifact-Portability-v1` | Supersede as an independent Stage stub | Retain source lineage and external dependency discovery; include only requirements independently selected by the current Stage contract. |
| `WP-1-Stage-ASR-Transcript-Lineage-v1` | Supersede as an independent Stage stub | Retain source lineage and external dependency discovery; include only requirements independently selected by the current Stage contract. |

No other packet or stub is authorized for absorption by the current operator direction.

## Requirement-level source-disposition map

| Requirement ID | Source | Type | Source requirement considered by current planning | Destination topic / lane | Future acceptance proof if selected |
|---|---|---|---|---|---|
| `STAGE-PRES-MVP-001` | Stage MVP | intent | Deliver a governed in-app browser that can capture/import external web, PDF, and 3D inputs without session bleed, origin confusion, or unlogged side effects. | renderer architecture; capture/import; security | End-to-end hostile-site and capture/import suite with durable receipts. |
| `STAGE-PRES-MVP-002` | Stage MVP | scope | External Web navigation remains a first-class Stage surface. | native frontend; renderer architecture | Real navigation, history, reload, error, download, and crash-recovery scenarios. |
| `STAGE-PRES-MVP-003` | Stage MVP | scope | Stage Apps use a dedicated trusted origin and are not treated as External Web. | Stage Apps and Bridge | Origin-classification tests across top frame, child frames, redirects, popups, and restored tabs. |
| `STAGE-PRES-MVP-004` | Stage MVP | scope / acceptance | Multiple Stage sessions isolate cookies, storage, cache, permissions, and service-worker state; no cross-session or cross-profile bleed. | sessions/profiles/state | Cross-profile negative suite, restart reconstruction, clear-data, export, and concurrent-use proof. |
| `STAGE-PRES-MVP-005` | Stage MVP | scope / acceptance | Privileged bridge access is available only to the trusted Stage-App origin; External Web is denied and every allow/deny is auditable. | Stage Apps and Bridge; security | Bridge-origin bypass tests plus EventLedger/Flight Recorder/internal diagnostics/Palmistry outcomes. |
| `STAGE-PRES-MVP-006` | Stage MVP | scope / acceptance | `stage.capture_webpage.v1` produces an evidence-grade current-page capture with content, visual evidence, manifest, SHA-256, provenance, and declared capture limitations. | capture/import | Authenticated current-renderer capture, cross-origin-frame cases, dynamic page cases, hash verification, and restart discovery. |
| `STAGE-PRES-MVP-007` | Stage MVP | scope / acceptance | `stage.clip_selection.v1` preserves selected content and stable source selectors linked to its originating capture. | capture/import; search/retrieval | Selection replay against unchanged and changed source snapshots, with explicit stale-selector outcomes. |
| `STAGE-PRES-MVP-008` | Stage MVP | scope / acceptance | `stage.import_pdf.v1` preserves exact PDF bytes and creates a linked document stub; structured conversion remains outside the initial slice. | capture/import | Byte/hash identity, provenance, malformed/encrypted PDF handling, and restart discovery. |
| `STAGE-PRES-MVP-009` | Stage MVP | scope / acceptance | glTF import and validation produce canonical scene-IR and validation/physics reports; the read-only viewport refuses unvalidated content. | capture/import; 3D assist | Malformed/bomb/over-budget/corrupt-asset tests and validation-gated viewport proof. |
| `STAGE-PRES-MVP-010` | Stage MVP | observability | Every privileged action is a durable Job/Workflow whose artifacts and evidence survive restart and are discoverable. | diagnostics/validation; artifact portability | Replay from PostgreSQL/EventLedger/ArtifactStore authority without chat, terminal, or Flight Recorder as state authority. |
| `STAGE-PRES-MVP-011` | Stage MVP | non-goal | Do not absorb Docling structured PDF conversion, a browser-extension ecosystem, third-party marketplace, bulk crawler/mirroring, Stage Studio authoring, or advanced 3D editing/collaboration into the initial Stage product. | scope and preservation | Packet non-goals and path/capability checks prevent scope leakage. |
| `STAGE-PRES-MVP-012` | Stage MVP | risk | Treat origin isolation, profile bleed, private-network/SSRF exposure, and missing manifest/hash/provenance as release-blocking failures. | security; acceptance/red-team | Negative security harness and evidence-integrity invariants. |
| `STAGE-PRES-PORT-001` | Media Artifact Portability | intent | One portable evidence contract spans Stage session/capture/import records, Media Downloader capture/auth/materialization outputs, debug bundles, export, replay, and storage portability. | media portability | Round-trip export/import across configured storage backends with invariant identity and lineage. |
| `STAGE-PRES-PORT-002` | Media Artifact Portability | scope | Session, authorization, materialization, and capture outputs are bounded export anchors with manifests and retention evidence. | media portability; credential handoff | Secret-redacted bundle inspection, retention/pin/GC proof, and explicit non-exportable fields. |
| `STAGE-PRES-PORT-003` | Media Artifact Portability | acceptance | Storage backend changes do not redefine artifact identity, meaning, provenance, retention, or replay semantics. | backend/data contracts | Backend conformance suite over canonical `ArtifactHandle` and manifest schemas. |
| `STAGE-PRES-PORT-004` | Media Artifact Portability | downstream contract | Loom, archive, and later integration packets can rely on stable Stage/media artifact and bundle-index semantics. | Loom integration; promotion/retirement | Consumer contract tests using only public handles/manifests, never Stage tables or renderer state. |
| `STAGE-PRES-PORT-005` | Media Artifact Portability | non-goal | Do not turn portability into full Stage UX or Media-to-Loom feature implementation. | scope and preservation | Work-packet boundaries and allowed paths keep consumer UI/feature work external. |
| `STAGE-PRES-PORT-006` | Media Artifact Portability | risk | Prevent Stage and Media Downloader from forking manifest, bundle-index, retention, or capture-session provenance semantics. | media portability | Schema compatibility, duplicate-source, partial-success, restart, and upgrade tests. |
| `STAGE-PRES-ASR-001` | ASR Transcript Lineage | intent / scope | Preserve one chain: Stage media artifact -> governed ASR job input -> transcript artifact -> searchable downstream consumer. | ASR lineage | End-to-end media-to-transcript-to-search proof with stable IDs and no copied media authority. |
| `STAGE-PRES-ASR-002` | ASR Transcript Lineage | scope | Preserve source hash, media-probe facts, Stage capture/session provenance, timing anchors, and transcript/source linkage. | ASR lineage; media portability | Hash/timing/probe invariants across retry, restart, duplicate input, and storage move. |
| `STAGE-PRES-ASR-003` | ASR Transcript Lineage | evidence | Progress, failure, cancellation, and transcript creation are recorder-visible while authoritative state remains reconstructable from PostgreSQL/EventLedger/ArtifactStore. | ASR lineage; diagnostics | Failure-path and replay proof without provider memory or UI state. |
| `STAGE-PRES-ASR-004` | ASR Transcript Lineage | downstream contract | Loom, Video Archive, Lens, and Studio can reuse one source-media/timing/provenance model. | ASR lineage; Loom integration | Contract fixtures consumed by each downstream boundary without field translation drift. |
| `STAGE-PRES-ASR-005` | ASR Transcript Lineage | non-goal | Do not absorb live captioning, diarization product work, full Lens transcript semantics, or Stage Studio UX. | scope and preservation | Packet non-goals and dependency graph keep those features external. |
| `STAGE-PRES-ASR-006` | ASR Transcript Lineage | risk | Transcript text surviving without timing, hashes, probe facts, or capture provenance is a failed result, not partial success. | ASR lineage; acceptance/red-team | Corruption and partial-write tests reject provenance-incomplete transcript promotion. |
| `STAGE-PRES-GOV-001` | all three stubs | lifecycle | The consolidated artifact remains `NON_EXECUTION_STUB` until refinement, operator confirmation/signature as required by current authority, official packet creation, and taskboard promotion. | promotion/retirement | Stub validator and taskboard/traceability/build-order checks. |
| `STAGE-PRES-GOV-002` | all three stubs | red team | Preserve the rule that stubs are planning metadata and Markdown projections are safety-net-only, not execution authority. | promotion/retirement | Contract/projection drift validation and launch refusal. |

## Legacy Master Spec behavior reselected by the current plan

The old platform-WebView implementation direction and all Stage-specific implementation shapes are superseded. The following behavior and safety outcomes remain only because the current plan independently selects them:

- Stage remains a browser/capture/import worksurface, not Studio authoring.
- External Web, trusted Stage Apps, trusted HTML artifacts, and privileged host/jobs are distinct trust zones; the trusted-HTML zone needs a new explicit contract.
- Network policy blocks localhost, private ranges, `file:`, and internal schemes by default; any local-project testing profile requires an explicit bounded capability, target binding, TTL, and evidence.
- Stage Bridge calls are top-frame/origin verified, schema/version/size/rate bounded, capability and approval gated, artifact-handle based, and forbidden from accepting raw filesystem paths.
- Stage App packages preserve manifest, content hash, CSP, install/update/revoke/rollback, compatibility, and crash-isolation obligations.
- Capture/import and 3D flows preserve quarantine, validation-before-use, resource budgets, provenance, and deterministic hostile-input fixtures.
- Platform-specific security knobs remain evidence inputs for the Chromium bootstrap, but Tauri/Wry/Electron are not retained as native-shell authority.

## Adjacent dependencies and consumers that remain separate

| External packet / system | Relationship the current Stage plan must honor |
|---|---|
| `WP-1-Media-Downloader-v2` | Active validated acquisition dependency; Stage supplies session-scoped authorization or governed cookie handoff and consumes versioned batch/control/result schemas. |
| `WP-1-Media-Downloader-Loom-Bridge-v1` | Downstream consumer of portable media/capture bundles. |
| `WP-1-ASR-Transcribe-Media-v1` | Governed ASR execution dependency. |
| `WP-1-Video-Archive-Loom-Integration-v1` | Downstream media/transcript/archive consumer. |
| `WP-1-Artifact-System-Foundations-v1` | Canonical `ArtifactHandle`, hashes, manifests, Materialize, retention, pinning, and garbage collection; Stage must not create a second artifact authority. |
| `WP-1-Storage-Trait-Purity-v1` | Backend-capability dependency; Stage owns no storage-backend fork. |
| `WP-1-Atelier-Lens-CKC-Core-Data-Intake-v1` and active `WP-CKC-posekit-overhaul` | Downstream project-intake and production consumers of canonical artifact/lineage records. |
| `WP-1-Session-Scoped-Capabilities-Consent-Gate-v1` | One Tool Gate across local, MCP, Stage Bridge, and model-driven transports; keep separate. |
| `WP-1-Mail-Runtime-Backfill-v1` | Preserve the Mail-to-Stage/media-capture handoff as an external edge. |
| `WP-KERNEL-012-Native-Editors-Obsidian-VSCode-Parity-v1` | Editor route-to-Stage and capture embed-back client; it does not own the browser product. |
| `WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1` | Dexterity model-lane, ToolGate, process recovery, promotion, and diagnostic provider; Stage does not build a second orchestration runtime. |

## Required consolidation transaction

After the operator approves the later proposal, the authoritative consolidation must be one coordinated transaction:

1. Update the destination machine-readable Stage stub while retaining `NON_EXECUTION_STUB` status.
2. Remove umbrella self-dependencies and independently revalidate every external dependency and downstream block selected by the current plan.
3. Update current-spec paths and anchors; do not retain `.GOV/roles_shared/SPEC_CURRENT.md` or v02.131/v02.150/v02.158 pointers.
4. Reconcile `user_signature_required: false` with the currently listed `obtain_user_signature` activation step instead of copying the contradiction.
5. Update traceability registry, taskboard, build order, source links, supersession links, and Markdown projections together.
6. Retain the two absorbed stubs and their projections under the repository archival workflow; do not delete them.
7. Validate contract/projection hashes and refuse promotion if any source-disposition row lacks a current-plan disposition, destination, or explicit rejection rationale.

</topic>
