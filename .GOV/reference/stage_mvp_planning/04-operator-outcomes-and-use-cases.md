---
file_id: stage-mvp-operator-outcomes-and-use-cases
file_kind: operator-requirement-preservation-note
updated_at: "2026-07-20"
status: active
wp_id: WP-1-Handshake-Stage-MVP-v1
---

<topic id="stage-mvp-operator-outcomes-and-use-cases" status="active" version="v0.2" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-20">

# Stage operator outcomes and use cases

## Preserved operator intent

The operator wants to start using Stage to find and download videos, images, and other assets for a project, then have Handshake ingest those assets for Lens/Atelier. Stage should also be a CPU-efficient browser for unusually large tab collections. The operator currently keeps more than 3,000 YouTube tabs containing AI videos and K-pop shows or performances, mostly as reminders or informal watch queues, and accepts slower tab restoration in exchange for substantially lower idle resource use.

The operator's feature notes are preserved below:

> - high volume tabs, tabs group folders per window (think of 3000+ tabs and folders per window) with low cpu usage.
> - tabs side bar per window
> - folders for grouped tabs
> - color labels
> - bookmarks
> - tags for folders (how can loom work/interact with this?)
> - cookie editor: export to json
> - steerable by local and cloud AI
> - visually navigationable and interactive so cloud and local ai can interact with captchas, websites, or self test web dev builts or gui mockups etc.
> - features and how to use them is included in handshake internal user manual
> - translation services for text/websites
> - export/save to MD, PDF
> - Fuzzy search for website content
> - fuzzy search for tabs. grouped tabs folders, bookmarks

Operator direction added 2026-07-20, preserved verbatim:

> - when downloading through stage this should never be stopped, freezed or put dormant
> - i feel you are overcomplicating everything. i just want to be able to have 3000+ tabs without it taxing my cpu. everything else a normal browser does should stay the same unless other handshake modules say otherwise.

These notes are locked as `STAGE-DEC-019` (unconditional download continuity) and `STAGE-DEC-020` (normal-browser baseline; complexity only where tab scale, a Handshake module, or security demands it) in the decision register.

## Primary operator workflows

### Project asset intake

1. Browse or search for a useful video, image, model, document, or other asset.
2. Select the page, media item, selection, playlist, channel, gallery, or asset link for download/capture.
3. Choose the destination Handshake project and intended consumer, such as Lens or Atelier.
4. Stage submits a governed Media Downloader or capture job using the current Stage session when authentication is required.
5. The job produces original bytes, source metadata, hashes, captions/transcripts when available, thumbnails/previews, and a portable manifest.
6. Stage Capture coordinates the operator workflow and records capture/intake lineage. Its `StageCaptureCoordinator` delegates governed bytes to the existing shared ArtifactStore, so Lens/Atelier receives shared artifact handles and provenance rather than unmanaged download paths. ArtifactStore is an internal dependency, not the Stage feature name.
7. The originating tab records the resulting artifact/job relationship and can be marked downloaded, ingested, watched, skipped, or still pending.

### Large reminder and watch queues

Tabs are not assumed to be active browser processes. Most are durable reminder records containing URL, title, favicon/thumbnail, folder, color, tags, bookmark state, project link, opened-from relationship, optional note, and last-known capture metadata.

A proposed lifecycle for later operator confirmation is:

- `ACTIVE`: visible page with a live renderer and full interaction.
- `SUSPENDED`: recently used live renderer with animation/timers throttled.
- `UNLOADED`: renderer destroyed; the tab remains in the sidebar and reloads when selected.
- `ARCHIVED`: removed from the active window but retained as searchable project/reminder knowledge.

The dominant resource-control mechanism should be aggressive suspension and unloading. HTTP cache, cookies, site storage, and saved-tab metadata are separate resources with separate retention policies. The shared HTTP/media cache should be bounded and evicted by quota/LRU policy rather than repeatedly erased merely because a tab is inactive.

Tabs playing audio, containing unsaved form state, running an operator-approved web test, or explicitly marked keep-live require visible, budgeted, expiring exemptions. Downloads are stronger than every other exemption per `STAGE-DEC-019`: an active download is never stopped, paused, frozen, or made dormant by any lifecycle mechanism. While a download is still renderer-bound, its carrier renderer is unconditionally exempt from suspension and unloading until the download completes or durable job handoff succeeds; after handoff the job is independent and the source tab is unloadable. Pin, bookmark, folder, and reminder status are metadata and never imply keep-live. Everything else may be unloaded aggressively because slow restoration is acceptable to the operator.

### Tab organization and Loom interaction

The proposed responsibility split is:

- Stage folders/groups provide fast per-window operational hierarchy.
- Color labels are lightweight Stage display metadata.
- Bookmarks are durable Stage link records independent of whether a renderer is live.
- Loom provides durable knowledge relationships: LoomBlocks, tags, mentions, backlinks, pins, project context, and semantic retrieval.
- Captured or downloaded content becomes an Artifact wrapped or referenced by a LoomBlock when promoted into project knowledge.
- A Stage folder may link to a Loom tag hub or saved Loom view, but folder hierarchy must not silently become a second copy of the Loom graph.
- Stage tags used for project knowledge should resolve to existing Loom tag identities where available rather than creating a competing tag namespace.

High-return workflow: Stage can detect large related tab sets, such as hundreds of YouTube videos from a channel or playlist, and offer to convert them into one governed downloader batch plus a Loom/project collection. This preserves every URL while eliminating the need to keep every page live.

### Local and cloud AI interaction

Models require two complementary paths:

- Structured interaction through DOM/accessibility snapshots, stable target identifiers, navigation state, selection state, and typed actions.
- Visual interaction through screenshots, OCR, pointer/keyboard actions, viewport control, and re-observation for canvas-heavy pages, GUI mockups, front-end testing, or other surfaces without usable semantics.

Every action must be attributable to the local/cloud model identity and pass Stage capabilities. Cloud-model observation must make external data egress visible. Site challenges such as CAPTCHAs must support clear operator takeover; Stage should not define a CAPTCHA-bypass subsystem. For self-owned or authorized test builds, the same visual interaction path can exercise the complete interface.

### Search, translation, and export

- Fuzzy search covers tab titles, URLs, folders, bookmarks, labels, notes, and tags without loading pages.
- Full-text search covers content already extracted or captured from pages.
- Optional semantic retrieval covers captured/indexed content and Loom relationships.
- Stage must not wake thousands of tabs merely to build or refresh an index.
- Translation can operate on selected text, readable page content, or captured artifacts while preserving the original source and recording which local or cloud service produced the translation.
- Markdown/PDF export should be generated from captured/sanitized content with source URL, capture time, provenance, and asset references rather than pretending a live dynamic page is a stable document.

### Cookies and authenticated downloads

Stage needs cookie inspection, editing, import, and JSON export. Because exported cookies are reusable credentials, raw export should require an explicit high-risk capability and native confirmation, be limited by session/domain selection, avoid Flight Recorder payload leakage, and default to encrypted materialization.

For Media Downloader, the preferred path is a scoped credential lease or direct governed cookie-jar handoff. This avoids creating a reusable JSON file for routine downloads. JSON remains available for explicit operator workflows and interoperability.

### Internal manual

The Handshake UserManual/model manual must explain Stage's purpose, tab lifecycle states, session isolation, folders/tags/Loom behavior, downloading and project ingestion, cookies, translation, export, AI control, safety boundaries, common failures, and recovery. Manual actions and examples should cite the same stable action IDs used by the model-facing Stage API.

## Initial acceptance implications

- A real 3,000+ tab window is a minimum operator-derived scale fixture, not an edge case.
- Idle CPU must not grow with unloaded-tab count as though every tab were live.
- Memory should scale primarily with the configured live/suspended renderer budget.
- The tab sidebar must be virtualized and searchable; it must not render thousands of tab rows simultaneously.
- Session restoration should initially restore records and only instantiate the selected/small working set.
- Download/capture jobs must continue without keeping their source tab live.
- No lifecycle mechanism may ever stop, pause, freeze, or make dormant an active download; renderer-bound downloads keep their carrier renderer alive until completion or durable handoff (`STAGE-DEC-019`).
- All other browser behavior defaults to normal-browser behavior unless another Handshake module's contract explicitly overrides it (`STAGE-DEC-020`).
- Search, folder operations, tags, bookmarks, and bulk selection must work on the canonical full tab set, not only currently rendered sidebar rows.
- Duplicate and canonicalized-URL detection should identify repeated reminders without deleting operator intent.

</topic>
