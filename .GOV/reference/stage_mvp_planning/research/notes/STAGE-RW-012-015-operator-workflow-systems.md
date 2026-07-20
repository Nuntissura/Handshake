---
file_id: stage-rw-012-015-operator-workflow-systems
file_kind: reference-research-note
updated_at: "2026-07-19"
research_workstreams:
  - STAGE-RW-012
  - STAGE-RW-013
  - STAGE-RW-014
  - STAGE-RW-015
verification_status: initial-primary-and-local-sources-checked
---

<topic id="stage-rw-012-015-operator-workflow-systems" status="initial" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Operator workflow systems research

## Questions

- How should the native sidebar scale to thousands of tab records?
- Which existing Handshake systems should Stage reuse for downloads, artifacts, project intake, Loom, and ASR?
- How should fuzzy/full-text search avoid waking pages?
- What field patterns support local translation, Markdown/PDF export, and cookie interoperability?
- What structured browser-control standard should inform the model-facing boundary?

## Findings

### Large native lists

egui's official `ScrollArea::show_rows` documentation explicitly provides a path that efficiently shows only the visible part of a large fixed-height row set and demonstrates 10,000 rows. This supports the proposed virtualized Stage sidebar. It does not by itself solve nested folder flattening, variable row height, accessibility, drag/drop, or canonical bulk-action correctness; those remain Stage proof obligations.

### Existing Handshake reuse

Local authority inspection found:

- active `WP-1-Media-Downloader-v2` is marked `Validated (PASS)` and supplies a unified queue plus versioned batch/control/result schemas;
- `WP-1-Artifact-System-Foundations-v1` is marked `Done` and owns canonical artifacts, hashes, Materialize, retention, pinning, and garbage collection;
- current Loom spec owns LoomBlocks, tags, mentions, backlinks, pins, and knowledge relationships;
- `WP-1-Atelier-Lens-CKC-Core-Data-Intake-v1` is a non-execution stub that explicitly carries Media Downloader/Loom/archive intake requirements into persistent project batches, collections, metadata, captions/transcripts, search, similarity, and provenance.

Therefore Stage should orchestrate these systems and add browser context/correlation. It should not create a second downloader, artifact store, graph, ASR pipeline, or consumer-specific media database.

### Agent interaction

The W3C WebDriver BiDi working draft defines bidirectional remote control of user agents with browsing contexts, scripts, navigation, events, network behavior, and node location including accessibility-based locators. It is an appropriate standards input for Stage's renderer-facing observation/action protocol. Handshake still needs its own capabilities, actor attribution, artifact references, receipts, parallel coordination, visual fallback, and no-foreground-interruption rules above that protocol.

### Search

PostgreSQL documents `pg_trgm` similarity operators/functions plus GiST/GIN index support and describes trigram matching as useful alongside full-text search for misspelled words. PostgreSQL also provides built-in full-text document/query/ranking facilities. This supports a split between fuzzy tab metadata and full-text captured content while reusing the project's PostgreSQL direction.

### Local translation

Firefox source documentation states that Firefox Translations uses a WASM version of Bergamot/Marian, quantized direction-specific models, HTML alignment, sentence iteration, and pivot translation where a direct pair is unavailable. This is a mature local-first pattern worth evaluating as a Stage service adapter. It is not evidence that all desired language pairs, quality levels, or resource targets are adequate; those require direct evaluation.

### Readable capture and export

Mozilla Readability returns article title, cleaned HTML, text, language, excerpt, author/site metadata, and published time. Its official repository warns that untrusted output needs a sanitizer and defense-in-depth such as CSP. Readability is therefore an extraction reference/candidate, not a complete safe export pipeline.

Chromium DevTools Protocol exposes `Page.printToPDF`; this can back the bootstrap adapter. Stage should still define renderer-neutral `Print page` and `Export captured document` semantics so Chromium does not become permanent architecture by accident.

### Cookies

The Cookie Store API living standard provides asynchronous cookie query/set/delete/monitoring concepts. yt-dlp's official FAQ requires Mozilla/Netscape cookie-file format for manual cookie input and warns that whole-browser exports expose cookies for every site. This supports separate, scoped paths: operator-requested JSON round-trip export and downloader-compatible Netscape adaptation. Routine downloader use should stay session/domain scoped and governed.

## Selected planning approach

- Native virtualized sidebar over canonical durable tab records.
- Existing Media Downloader v2 and ArtifactStore/Materialize as acquisition and byte authorities.
- Existing project intake, Loom, and ASR contracts as downstream authorities.
- PostgreSQL trigram search for fuzzy metadata plus PostgreSQL full-text search for captured text, subject to product topology verification.
- Local-first translation adapter evaluation based on Bergamot field patterns, with explicit cloud egress.
- Captured/sanitized source as the basis for Markdown/stable-document PDF; renderer print is a separate capability.
- Structured model interaction informed by WebDriver BiDi plus visual fallback and Handshake governance.
- Cookie JSON export and downloader Netscape adaptation remain separate, scoped, secret-safe workflows.

## Rejected options

- Rendering all tab rows or keeping one webview per tab: fails the operator's resource goal.
- Clearing all cache on every unload: attacks disk cache rather than live page execution and increases restore traffic.
- A new Stage-specific downloader or artifact store: duplicates validated/finished Handshake systems.
- Reusing Loom tags as a hidden copy inside Stage: creates divergent authority.
- Waking pages to index them: converts search into CPU/network load and changes page state.
- Chromium-only print/export semantics: makes bootstrap behavior define the product contract.
- Treating Readability output as sanitized: contradicted by its own security guidance.
- One cookie export format for all uses: JSON round-trip and yt-dlp Netscape input have different interoperability contracts.

## Risks and mitigations

- `pg_trgm` ranking may not match operator expectations: expose rank reasons and tune against a representative 3,000-tab query corpus.
- Local translation coverage/quality may be insufficient: publish installed pair coverage, benchmark, and allow explicit governed adapters.
- Captured content may be incomplete on dynamic/canvas pages: retain original capture/screenshot provenance and label extraction limits.
- WebDriver BiDi concepts may not map directly to Servo APIs: keep a Stage-domain protocol and implement adapters with explicit capabilities.
- Existing runtime may differ from governance contracts: inspect the product worktree before module/schema/file decisions.

## Sources checked

- `STAGE-SRC-LOCAL-006` through `STAGE-SRC-LOCAL-009`.
- `STAGE-SRC-WEB-015` through `STAGE-SRC-WEB-025`.

## Validation plan

- Prototype/benchmark the sidebar with the 3,000+ fixture and canonical offscreen bulk actions.
- Inspect product source for the actual downloader, artifact, intake, Loom, ASR, PostgreSQL, event, UI, and command entry points.
- Build a query corpus from realistic tab titles/URLs/folders/notes and measure exact/fuzzy/full-text behavior.
- Evaluate local translation language coverage, quality, memory, CPU, cold-load, and offline operation.
- Run malicious HTML through extraction/sanitization/export tests.
- Round-trip cookie attributes per renderer and prove secret-free logs/receipts.
- Run the same structured/visual agent fixtures through Servo and bootstrap Chromium capability manifests.

</topic>
