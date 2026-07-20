---
file_id: stage-mvp-loom-search-translation-export-and-manual
file_kind: reference-integration-plan
updated_at: "2026-07-19"
status: proposed
wp_id: WP-1-Handshake-Stage-MVP-v1
---

<topic id="stage-mvp-loom-search-translation-export-and-manual" status="proposed" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Loom, search, translation, export, and manual integration

## Planning outcome

Stage organization should remain fast and operational at browser scale while Loom remains the durable knowledge graph. Search should cover tab metadata without loading pages and captured content without pretending it is live. Translation and document export should produce derived, provenance-linked artifacts. The shipped manual should expose the same actions and states used by operators and models.

## Stage and Loom ownership

| Concern | Stage owns | Loom owns | Link behavior |
|---|---|---|---|
| Window organization | folders, ordering, saved views, active/archive state | none | optional folder-to-Loom-view reference |
| Lightweight visual classification | color labels and browser workflow status | none | may be projected into a Loom view, not copied as authority |
| Bookmark/reminder | durable URL record, note, watch status | project knowledge only when promoted | bookmark may reference a LoomBlock |
| Knowledge tags | resolves/uses Loom tag identities | tag blocks and tag relationships | Stage displays and queries Loom tag IDs |
| Mentions/backlinks | link indicator and navigation | authoritative relationships | Stage follows Loom references |
| Captured/downloaded content | source tab/job/artifact correlation | LoomBlock/collection context | promotion links ArtifactHandle and source tab |
| Semantic retrieval | sends bounded source/filters | graph/retrieval contracts | results retain Loom IDs and citations |

A Stage folder may represent an operator queue such as `K-pop / watch next`; a Loom tag represents knowledge that can span browsers, documents, assets, and projects. They may be linked, but automatic bidirectional mirroring would create drift and should not be the default.

## Search model

### Tier 1: canonical tab and organization search

This path searches durable metadata without renderer activation:

- title, original/normalized URL, domain, folder path, bookmark name;
- color label, reminder note, workflow state, project, intended consumer;
- Loom tag names/IDs and linked collection/Block titles;
- opened-from/watch-after relationships and timestamps.

PostgreSQL `pg_trgm` is a strong candidate for typo-tolerant/fuzzy matching because it provides similarity functions/operators and GiST/GIN index support. Exact URL/domain/prefix and structured filters should remain separate deterministic clauses. Ranking should be explainable as field weights plus match evidence, not a hidden model-only score.

### Tier 2: captured-content full-text search

This path searches content that Stage has already captured or extracted:

- readable page text and metadata;
- explicit selection clips;
- imported PDF/document text;
- captions and transcripts;
- translation artifacts, clearly labeled as derived;
- capture revision, source URL, time, and ArtifactHandle.

PostgreSQL full-text search is the initial field-aligned candidate. Indexing must consume captured artifacts or explicit bounded jobs. It must not wake unloaded tabs or silently refetch the web.

### Tier 3: optional semantic and graph expansion

Semantic retrieval should reuse Handshake/Loom retrieval contracts over artifacts and LoomBlocks. It is for discovery and relationship expansion. A request for a known tab, artifact, job, or LoomBlock ID should use direct structured lookup rather than unnecessary RAG.

### Search result truth labels

Every result should identify whether its text is:

- current durable tab metadata;
- last-observed live-page metadata;
- captured original content;
- derived readable text;
- transcript/caption;
- translation;
- Loom-authored/project knowledge.

Staleness and capture time must be visible. Search must not present an old capture as the current live page.

## Content extraction and Markdown export

Mozilla Readability is a relevant proven behavior/reference for deriving article title, cleaned HTML, text, language, author/site metadata, and excerpt. Its own documentation warns that output from untrusted input still requires sanitization and defense-in-depth. Stage should define a renderer-independent extraction contract and evaluate an appropriate maintained implementation during the implementation research lane.

Proposed pipeline:

1. Freeze source URL, time, page revision, renderer, session classification, and capture policy.
2. Capture original/sanitized DOM or snapshot according to the capture contract.
3. Derive readable HTML/text where applicable; keep failure explicit for non-article pages.
4. Sanitize active content and rewrite embedded asset links to governed artifacts where captured.
5. Convert the sanitized representation to Markdown without discarding the original capture.
6. Store the Markdown and its manifest in ArtifactStore.
7. Materialize only through Export/Materialize with an ExportRecord.

Markdown output should include source URL, title, capture time, author/site metadata when available, source ArtifactHandle/hash, extraction version, and asset references. It must not imply pixel-perfect archival fidelity.

## PDF export

PDF has two distinct intents and the UI should name them:

- `Print page`: renderer print layout, where supported;
- `Export captured document`: a stable PDF generated from sanitized captured/readable content.

Chromium's DevTools protocol exposes `Page.printToPDF`, which can support the bootstrap adapter. That API must not define the Stage-level PDF contract. Servo requires a proven common path or an explicit capability limitation until native print support meets conformance. No silent engine fallback is allowed.

Both PDF paths should preserve source/provenance in the artifact manifest and use Export/Materialize for external copies.

## Translation

Stage should offer:

- translate selected text;
- translate readable page content;
- translate a captured artifact;
- side-by-side source/translation viewing;
- optional cloud service adapters behind visible egress and redaction controls.

Firefox's current translation system is a useful local-first reference: it uses a WASM build of Bergamot/Marian, quantized direction-specific models, HTML alignment, and optional pivot translation. This supports a practical offline candidate without making translation part of the renderer engine.

Proposed contract:

- original text/artifact is immutable and remains the primary source;
- translation is a derived artifact linked to the exact source revision;
- record source/target language, provider, model/version, local/cloud mode, pivot path if used, time, and segment alignment where available;
- do not auto-send private/project content to a cloud translator;
- model availability, download size, CPU/memory use, and quality vary by language pair and require benchmark/evaluation before default enablement;
- page translation changes the presentation layer or a derived document, never overwrites captured source text.

## Cookie editor and explicit JSON export

The Stage Session drawer should provide cookie list/filter/create/edit/delete/import/export behavior while making scope visible. A canonical JSON export proposal should preserve cookie attributes needed for round-trip where supported, including name, value, domain, path, expiry, secure, httpOnly, sameSite, partitioning metadata, and source session/created time outside the cookie payload as provenance.

The Cookie Store API living standard provides query, set, delete, and monitoring concepts, but browser internals and interoperability formats may expose additional attributes. The final JSON schema therefore requires renderer-by-renderer round-trip tests.

Export rules:

- explicit high-risk action and native confirmation;
- default scope is selected session plus selected domain(s), never every profile silently;
- preview shows cookie names/domains/counts but masks values;
- encrypted artifact/materialization by default;
- values excluded from Flight Recorder, search, screenshots, crash reports, and model observations;
- separate adapters for JSON and existing downloader-compatible Netscape format;
- import is a previewed mutation with conflict, expiry, domain, and unsupported-attribute reporting.

## Internal operator and model manual

Stage should ship one navigable manual corpus with operator and no-context-model views over the same action/state registry. It must cover:

- purpose, startup, window/session creation, navigation, and renderer provenance;
- tab lifecycle, live budgets, keep-live exemptions, large-session recovery, and resource diagnostics;
- sidebar folders, labels, bookmarks, Loom tags/links, watch queues, and bulk actions;
- capture/download/project-intake workflows, existing Media Downloader behavior, jobs, results, and recovery;
- cookie editing/export/import and secret-handling boundaries;
- search truth labels, translation provenance, Markdown/PDF meanings, and export paths;
- structured and visual model interaction, capabilities, receipts, operator takeover, and CAPTCHA handling;
- common failures: renderer crash/hang, expired session, unsupported media, partial batch, disk full, intake unavailable, corrupt profile, stale target, and failed restore;
- diagnostics and exact recovery commands/actions.

Every actionable manual step should reference stable action IDs and expected input/output records. Example workflows should be executable against a safe fixture site and should not depend on hidden conversation history.

## High-ROI additions while this area is open

- Saved watch-queue views reuse tab metadata and eliminate thousands of manually live reminders; validate with canonical-query and ordering tests.
- Duplicate/canonical URL grouping reuses search normalization and prevents redundant downloads without deleting intent; validate with distinct-note/folder cases.
- Source-versus-derived truth labels reuse artifact provenance and prevent stale/translated text confusion; validate across live, captured, transcript, and translation results.
- One action registry for UI, model API, manual, receipts, and tests reduces drift; validate that every shipped action has documentation, capability, input/output, and fixture coverage.
- Local-first translation reuses a mature field pattern and reduces cloud egress; validate language coverage, quality, model resource use, and offline behavior before defaulting.

## Sources and reuse anchors

- `STAGE-SRC-LOCAL-008`: LoomBlock/tag/mention/backlink/pin behavior and current storage direction.
- `STAGE-SRC-WEB-018`: PostgreSQL `pg_trgm` fuzzy matching and indexes.
- `STAGE-SRC-WEB-019`: PostgreSQL full-text search.
- `STAGE-SRC-WEB-022`: Mozilla Readability API and security guidance.
- `STAGE-SRC-WEB-023`: Chromium `Page.printToPDF` bootstrap capability.
- `STAGE-SRC-WEB-024` and `STAGE-SRC-WEB-025`: Firefox/Bergamot local translation architecture.
- `STAGE-SRC-WEB-021`: Cookie Store API living standard.

</topic>
