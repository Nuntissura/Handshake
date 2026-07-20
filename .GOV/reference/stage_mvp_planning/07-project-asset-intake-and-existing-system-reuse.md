---
file_id: stage-mvp-project-asset-intake-and-existing-system-reuse
file_kind: reference-integration-plan
updated_at: "2026-07-19"
status: proposed
wp_id: WP-1-Handshake-Stage-MVP-v1
---

<topic id="stage-mvp-project-asset-intake-and-existing-system-reuse" status="proposed" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Project asset intake and existing-system reuse

## Planning outcome

The first operator-valuable Stage workflow is browsing to useful media or assets and sending them into a Handshake project for Lens/Atelier. The Stage-facing name is **Stage Capture** and the technical orchestration component is `StageCaptureCoordinator`. Stage should provide the browser context, selection, session, project destination, status, and provenance while reusing existing download, artifact, workflow, export, ASR, Loom, and intake systems.

Stage must not implement a second media downloader. The active `WP-1-Media-Downloader-v2` packet is validated `PASS` as a work contract, but direct current-product inspection found that `atelier/downloader.rs` is records-oriented while the executing v0 workflow remains in `workflows.rs`, reads `ArtifactHandle.path`, uses a legacy Stage-session registry file, and owns a separate streaming artifact writer. The dependency is therefore not production-ready reuse: `STAGE-DEP-MEDIA-DOWNLOADER` must bind one versioned request/control/result implementation, scoped credential leases, the canonical opaque shared ArtifactHandle, and the shared streamed ArtifactStore contract before Stage production integration.

## Existing contracts to consume

### Media Downloader v2

Stage should call the existing versioned request/control/result schemas:

- `hsk.media_downloader.batch@v0`;
- `hsk.media_downloader.control@v0`;
- `hsk.media_downloader.result@v0`.

The existing contract already covers YouTube video/playlist/channel input, Instagram input where accessible, forum/blog image crawling, generic video input, queueing, progress, pause/resume/cancel/retry, concurrency, captions, source metadata, hashes, OutputRootDir materialization, and Stage Session or cookie-jar authentication.

Stage adds the operator entry point and contextual orchestration. It does not fork these schemas merely to add a browser button.

### Stage Capture and the shared Artifact System

`StageCaptureCoordinator` owns capture intent, source/tab/session correlation, job submission, completion reconciliation, Stage capture lineage, and downstream handoff. It consumes shared opaque `ArtifactHandle`s and does not expose ArtifactStore as the Stage feature name.

ArtifactStore remains global Handshake infrastructure and owns governed bytes, manifests, hashes, retention, pinning, and garbage-collection behavior. Materialize/Export owns external filesystem copies and `ExportRecord.materialized_paths[]`. Lens/Atelier should receive ArtifactHandles and lineage first; an OutputRootDir path is an optional materialization, not the primary identity of an asset. Stage must not scan the ArtifactStore per tab, per sidebar refresh, or during ordinary startup; store maintenance remains independently scheduled, bounded background work.

Current product inspection shows a production integration gap: `ArtifactWriteEntry` owns a complete `Vec<u8>`, file writes accept a complete byte slice, directory writes retain entries in memory, and materialization is also byte-entry-oriented in `../handshake_main/src/backend/handshake_core/src/storage/mod.rs`. That is suitable for small evidence payloads but is not yet proven for large media, PDF, WARC, or multi-part renderer capture. Before Stage production use, the shared Artifact System needs a versioned streamed-ingest boundary with:

- begin/append/finalize/abort lifecycle and an expiring ingest lease;
- incremental SHA-256 while bytes stream, bounded buffers, backpressure, cancellation, and progress coalescing;
- atomic idempotent finalization with expected hash/size checks and an `ArtifactHandle` plus observed size/MIME/hash result;
- orphan-staging cleanup and reconcile-after-uncertain-finalize behavior;
- bounded/range reads and materialization by handle rather than caller-supplied whole-payload buffers;
- optional physical deduplication that preserves distinct logical provenance instead of merging operator records.

This is shared Artifact System hardening required by Stage; Stage must not implement a parallel streaming store to work around it.

### Atelier/Lens intake

The current intake stub carries forward persistent batches, accept/reject/defer, idempotency, restart/resume, collections/contact sheets, source metadata, captions/transcripts, tags/search/similarity, and no-UI-lock batch behavior. Stage should submit to that intake boundary rather than writing consumer databases or indexes itself.

### Loom and ASR

- Loom owns knowledge relationships, tags, mentions, backlinks, pins, and collection context.
- ASR owns transcription work and timing-aware transcript artifacts.
- Stage links outputs and status to the source tab but does not become the authority for either system.

## Primary workflow: current page or media

1. The operator chooses `Download`, `Capture`, or `Send to project` from the active page, context menu, selection, detected media list, or tab batch.
2. Stage freezes a bounded source context: tab ID, exact URL, normalized URL, title, source selection/media candidate, Stage Session ID, current renderer, timestamp, and project destination.
3. Stage presents the normalized plan before expensive work: source kind, number of candidates, destination project/consumer, authentication mode, captions/transcript policy, output/materialization policy, expected risks, and duplicate findings.
4. For supported web-media acquisition, Stage submits `hsk.media_downloader.batch@v0` through the existing workflow path.
5. For page/selection/document/3D capture, Stage submits the applicable existing or Stage capture/import job; it does not disguise every capture as a download.
6. The job detaches from the browser renderer. The source tab may suspend, unload, close, or crash without cancelling the job.
7. The shared ArtifactStore finalizes result bytes and manifests with source URL, retrieval/capture time, hashes, media facts, captions/transcripts, errors, and renderer/session provenance as allowed by policy; `StageCaptureCoordinator` receives only finalized handles or an explicit partial/failure disposition.
8. A persistent project intake batch receives the artifact handles and context. Lens/Atelier can accept, reject, defer, group, index, or derive previews without blocking Stage UI.
9. Stage updates the source tab/selection with job and artifact references plus `downloaded`, `ingested`, `partial`, `failed`, or `needs-review` state.
10. Optional Loom promotion creates or links the relevant LoomBlock/collection relationships without copying the Loom graph into Stage.

## Proposed Stage intake context

Stage needs a small correlation envelope around existing jobs. The name and schema are provisional; it must not break the existing downloader contract.

Candidate fields:

- `stage_request_id`, `window_id`, `tab_id`, and optional selected-tab IDs;
- `stage_session_id` and renderer provenance without cookie values;
- source URL, normalized URL, title, selection/media-candidate reference, and capture timestamp;
- destination workspace/project and intended consumer(s);
- requested operator workflow state after completion;
- existing LoomBlock/folder/tag/collection references;
- duplicate-policy decision and idempotency key;
- linked downloader/capture/intake/ASR job IDs and ArtifactHandles as they become available.

This envelope may live as a workflow correlation record or an intake request, depending on current product code. It should not be added as a new authoritative schema until that topology is inspected.

The Stage data model should normalize arbitrary results rather than place unbounded handle/job arrays on `stage_tabs` or one capture row. `stage_capture_parts` records capture-part role, shared handle, observed hash/size/MIME snapshot, completeness, derivation, and state. `stage_source_artifact_links` records the source tab/selection, capture or workflow run, shared handle, relation (`captured`, `downloaded`, `thumbnail`, `transcript`, `translation`, `export`), source revision, idempotency, and timestamps. These are Stage lineage records, not duplicate ArtifactStore manifests.

## Batch conversion for large tab sets

Stage should turn the operator's large reminder collections into bounded jobs without erasing the reminder system:

1. Select a folder, saved view, playlist/channel group, or arbitrary tab set.
2. Query canonical tab IDs and URLs, including unloaded tabs.
3. Normalize and group by supported source kind.
4. Detect exact/canonical URL duplicates and existing artifact/import matches.
5. Show a plan separating already ingested, queued, unsupported, authorization-required, and ready items.
6. Submit one or more existing Media Downloader batches using bounded item/concurrency limits.
7. Keep a stable mapping from each input tab to batch item/result.
8. Offer a project collection/contact-sheet and Loom collection projection.
9. Preserve source tabs until the operator chooses archive/close; never treat successful download as permission to delete reminders automatically.

## Authentication and cookie handoff

The existing downloader contract supports `none`, `stage_session(stage_session_id)`, and `cookie_jar(artifact_ref)`. Stage should prefer the `stage_session` path for routine authenticated downloads:

- the operator chooses an existing isolated Stage Session;
- a host-side governed exporter derives only the required domain cookies into the downloader-compatible path;
- any Netscape cookie artifact is high sensitivity, `exportable=false`, never sent to OutputRootDir, and deleted/expired according to a short retention policy;
- cookies, tokens, and authorization headers are absent from logs, events, receipts, search, and screenshots;
- the job records session and domain provenance without recording secret values.

The active Media Downloader refinement currently specifies a host-only Stage Session to Netscape `cookies.txt` artifact adapter. That older Stage connector is inspected evidence, not a compatibility obligation. The yt-dlp FAQ confirms that its manual cookie input requires Mozilla/Netscape format. If the current Stage plan selects this downloader path, it must implement a new governed, scoped Netscape-format boundary; Stage's operator-requested JSON export remains a separate explicit interoperability feature.

A current-design in-memory or short-lived scoped credential lease may reduce reusable-cookie materialization further. Selection is based on the new security/session contract, not compatibility with the superseded adapter.

## Non-media assets and page discovery

Stage should distinguish:

- browser-native downloads initiated by navigation or a link;
- detected media candidates from page/network inspection;
- images/assets selected from the DOM;
- article/page capture;
- PDF import;
- direct model/document/archive links;
- 3D import and validation;
- playlist/channel/gallery/forum expansion delegated to existing job types.

Detection is a proposal surface. The operator should see the exact resolved URL, media kind, credentials needed, estimated scope, and destination before a large or privileged job is accepted.

## Idempotency and provenance

- Every submission needs a stable request/idempotency key.
- Duplicate detection should consider source stable IDs, normalized URL, content hash, destination project, and prior result state.
- A repeated request may link to the existing artifact, retry only failed items, or create a distinct capture revision; it must not silently duplicate or overwrite.
- Original, captured, downloaded, transcoded, caption, transcript, translated, thumbnail, and export artifacts retain explicit derivation edges.
- Partial success is first-class: successful items remain usable while failures retain bounded reasons and retry controls.
- Source tab status is a projection of job/intake evidence, never a substitute for canonical job or artifact state.

## Failure scenarios and hardening

| Failure | Required behavior |
|---|---|
| Source tab unloads or renderer crashes | Detached job continues; source context remains frozen and linked |
| Session expires | Job pauses/fails with re-auth action; no automatic credential broadening |
| Playlist expands far beyond expectation | Bounded plan, max-item policy, backpressure, and operator confirmation |
| Disk becomes full | Job stops safely, preserves completed artifacts/results, and offers recover/retry |
| Duplicate already exists | Show match and allow link, skip, retry, or new revision |
| One item fails in a batch | Preserve other successes and item-level error/retry evidence |
| Consumer intake unavailable | Keep ArtifactHandles and pending intake batch; no redownload required |
| Raw cookie value reaches telemetry | Fail redaction tests and block completion |
| Unsupported page/media | Offer capture/link/bookmark fallback explicitly; never fake a successful download |

## Acceptance implications

- The Stage button path must demonstrably create the existing downloader request schema and consume the existing result schema.
- A source tab can unload/close while the job and project intake continue.
- Every successful asset is addressable by ArtifactHandle and hash before optional materialization.
- A 3,000-tab canonical query can create a bounded batch plan without opening the pages.
- Authenticated download tests prove domain/session scoping and secret-free logs/receipts.
- Captions are preferred when available; ASR derivation preserves source/transcript lineage when captions are absent or explicit retranscription is requested.
- Partial failure, restart/resume, cancellation, deduplication, and consumer-unavailable recovery are tested end to end.
- Product-code inspection must prove the actual v2 downloader schemas and ArtifactStore/intake entry points, then define the new Stage-side connector without inheriting the older Stage Session adapter.

## Sources and reuse anchors

- `STAGE-SRC-LOCAL-006`: active validated Media Downloader v2 packet/refinement.
- `STAGE-SRC-LOCAL-007`: Artifact System Foundations packet.
- `STAGE-SRC-LOCAL-008`: current Loom contract in the master spec.
- `STAGE-SRC-LOCAL-009`: Atelier/Lens CKC Core Data Intake stub.
- `STAGE-SRC-WEB-020`: yt-dlp cookie format and risk guidance.
- `STAGE-SRC-WEB-021`: Cookie Store API standard for cookie query/mutation concepts.

</topic>
