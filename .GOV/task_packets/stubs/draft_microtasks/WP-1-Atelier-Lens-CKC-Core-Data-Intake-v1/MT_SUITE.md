---
file_id: mt-suite-wp-1-atelier-lens-ckc-core-data-intake-v1
file_kind: draft_microtask_suite
updated_at: 2026-05-16
status: draft_non_executable
wp_id: WP-1-Atelier-Lens-CKC-Core-Data-Intake-v1
official_microtasks_generated: false
replaces_inline_twenty_task_draft: true
---

<topic id="draft-microtask-suite" status="draft" version="v1" wp="WP-1-Atelier-Lens-CKC-Core-Data-Intake-v1" summary="Fresh no-context draft microtasks for Core Data Intake" updated_at="2026-05-16">

# Draft Microtask Suite: Core Data And Intake

These microtasks replace the prior 20-item draft list. They are not executable until the stub is activated into an official signed work packet with refinement, packet contract, and generated `MT-*.json` / `MT-*.md` contracts. Do not reuse the old 20-task draft as source material except as negative evidence of insufficient granularity.

Each MT below is intentionally small. A future no-context local model or smaller cloud model should be able to implement one MT after reading only the activated packet, this MT, and the files named by the MT.

## Shared Execution Rules For Every MT

- Authority: planning draft only; no coder or validator may start from this file before activation.
- Runtime translation: implement in Handshake primitives, not CKC/Electron/SQLite runtime.
- Storage rule: PostgreSQL/EventLedger/ArtifactStore authority only; SQLite is forbidden in runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, and product paths.
- Path rule: no hardcoded machine-local paths; use repo-root, artifact-root, OutputRootDir, or operator-configured roots.
- Evidence rule: every mutation or job produces EventLedger/Flight Recorder evidence and a recoverable artifact/receipt where applicable.
- Status rule: CKC `REVIEW`, `BLOCKED`, and `PLANNED` rows are not DONE parity. Preserve their status in acceptance and fixtures.

### MT-001 - Source Evidence And Status Maturity Matrix
- Objective: Build the activation source matrix for Core/Data so later MTs know which source facts are DONE, REVIEW, BLOCKED, PLANNED, stale, or unresolved.
- Inputs: this stub; `.GOV/reference/ckc_atelier_lens_consolidation/*`; CKC taskboard; CKC spec v00.075; CKC README stale v00.063 pointer; old Handshake stubs named in this packet.
- Work slice: create/update the official packet's source matrix only; do not implement product behavior.
- Outputs: a table with source path, source anchor, requirement, owner stub, evidence maturity, activation action, and unresolved-source flag.
- Acceptance: every EVOL/OVR row owned by Core appears once; `WP-1-Atelier-Lens-v2`, `WP-1-Photo-Studio-v2`, Media Downloader v1/v2, ASR, Loom, and CKC WP-0122..WP-0137 are represented.
- Verification: run the packet truth-bundle check chosen at activation and manually compare row counts against the repaired stub.
- Depends on: none.

### MT-002 - Product Anchor Verification
- Objective: Verify the actual Handshake product files/modules that will receive Core/Data behavior.
- Inputs: activated packet; `.GOV/spec/SPEC_CURRENT.md`; `.GOV/spec/HANDSHAKE_PRODUCT_REFERENCE.md`; product worktree path selected by Orchestrator.
- Work slice: inspect product code and replace candidate module names with verified paths or `BLOCKED_MISSING_ANCHOR`.
- Outputs: anchor map for data models, Tauri/Rust APIs, React projections, ArtifactStore, EventLedger, Flight Recorder, search/index, export/jobs, and tests.
- Acceptance: no later MT names an unverified path unless it explicitly blocks on `BLOCKED_MISSING_ANCHOR`.
- Verification: `rg --files` evidence and an anchor-map JSON/Markdown projection in the official packet folder.
- Depends on: MT-001.

### MT-003 - Core Domain Boundary Skeleton
- Objective: Define the Core/Data bounded modules before writing behavior.
- Inputs: MT-002 anchor map; repaired stub `HANDSHAKE_TRANSLATION`.
- Work slice: create or update interface/skeleton files only for `atelier_core`, `atelier_sheet`, `atelier_media`, `atelier_intake`, `atelier_collections`, `atelier_search`, `atelier_exports`, and `kernel_event_bridge`.
- Outputs: minimal traits/types/module files plus compile-only tests if the product stack supports them.
- Acceptance: module boundaries compile and contain no business logic beyond type declarations and explicit TODO-free method signatures.
- Verification: product format/typecheck/build command selected at activation.
- Depends on: MT-002.

### MT-004 - Runtime Rejection Tripwires
- Objective: Add validation that rejects CKC runtime assumptions before feature code begins.
- Inputs: runtime rejection map in the stub.
- Work slice: add checks/tests for SQLite, Electron authority, CKC/CastKit runtime namespace, localhost intake authority, `.GOV` product outputs, and machine-local absolute paths.
- Outputs: validator/check script or test cases wired into the WP validation plan.
- Acceptance: a deliberate forbidden string or fixture path fails the check; legitimate historical evidence references remain allowed only under `.GOV/reference`.
- Verification: run the new negative and positive check cases.
- Depends on: MT-002.

### MT-005 - Core Event Family Baseline
- Objective: Define EventLedger/Flight Recorder event families used by later Core/Data MTs.
- Inputs: EventLedger and Flight Recorder anchors from MT-002.
- Work slice: add event names and minimum payload schemas for character, sheet, media, intake, collection, contact sheet, doc, moodboard, relation, search, export, backup, reset, and recovery actions.
- Outputs: schema/types/tests or packet contract rows, depending on product architecture.
- Acceptance: every later mutation MT can cite an event family and minimum payload fields.
- Verification: schema validation/unit test or generated event catalog check.
- Depends on: MT-003.

### MT-006 - Stable Character Identity Model
- Objective: Implement stable public character IDs separate from internal storage IDs.
- Inputs: CKC `Character.public_id`, public-id helpers, backend public character ID tests.
- Work slice: add character identity table/type fields and ID generation/lookup rules.
- Outputs: public ID create/read/search behavior with internal ID never exposed as public label.
- Acceptance: public IDs survive rename/import/export; filenames/events do not leak internal IDs or sensitive fields.
- Verification: tests for create, rename, lookup by public ID, duplicate prevention, and export manifest.
- Depends on: MT-003, MT-005.

### MT-007 - Identity-Decoupled Artifact Naming
- Objective: Prevent character names and sensitive sheet fields from entering generated file paths/events.
- Inputs: CKC backend identity-decoupling test and content-hash naming behavior.
- Work slice: add no-space content-addressed naming utility or integrate with ArtifactStore naming.
- Outputs: naming function and tests for character images, contact sheets, backups, share packs, and sidecars.
- Acceptance: generated paths contain stable hashes/ids but not display names, raw prompt text, or sheet values.
- Verification: tests using explicit sensitive sample names and sheet text.
- Depends on: MT-006.

### MT-008 - Template AST And Typed Field Contract
- Objective: Preserve CKC typed template parser behavior without copying CKC runtime.
- Inputs: `templateParser.js`, `sheet.js`, `validation.js`, template parser union tests, validation field type tests.
- Work slice: define AST/domain types for fields, unions, enum-with-other descriptors, score normalization, optional values, and block fields.
- Outputs: parser contract or parser skeleton plus fixtures.
- Acceptance: descriptor fallback and union parsing are explicit; malformed templates produce structured errors, not silent drops.
- Verification: fixtures for `<string | unset>`, `<integer | adult>`, `<score_10 | optional>`, enum `other:<descriptor>`, and invalid descriptor.
- Depends on: MT-003.

### MT-009 - Block List Field Contract
- Objective: Preserve recursive block-list editing/storage semantics as data behavior.
- Inputs: CKC `BlockListEditor.tsx`, block-list validation tests, character template block fields.
- Work slice: implement storage/validation support for block lists and nested block fields; UI editor can be deferred.
- Outputs: domain validation and serialization fixtures for repeated blocks.
- Acceptance: block rows round-trip with stable order; validation paths include row index and sub-field id.
- Verification: tests for add, remove, reorder, invalid nested score, and byte-preserved note field.
- Depends on: MT-008.

### MT-010 - Parser Unmapped Text Preservation
- Objective: Preserve raw/unmapped lines instead of rewriting or dropping user text.
- Inputs: CKC `SheetFieldChangePicker`, parser unmapped behavior, no-rewrite requirement.
- Work slice: add raw-line capture and structured unmapped field records to sheet parsing/merge.
- Outputs: unmapped record type, parse result field, and merge preview display contract.
- Acceptance: unknown field IDs and malformed lines are carried forward with source bytes and never silently deleted.
- Verification: test malformed template, unknown field id, and user text with explicit punctuation/newlines.
- Depends on: MT-008.

### MT-011 - Protected Field And Role-Scope Edit Guard
- Objective: Preserve protected fields and selection/range-bounded apply safety.
- Inputs: CKC `ProtectedField`; Atelier Collaboration Panel range-bounded patch precedent.
- Work slice: add protected-field metadata and validation preventing model/import edits from touching protected values unless authorized.
- Outputs: guard in sheet apply/revert/import path and event payload for rejected attempts.
- Acceptance: protected fields are visible in diff/preview but default unselected; out-of-range or protected changes are rejected with evidence.
- Verification: tests for protected apply rejection, authorized override, and EventLedger rejection record.
- Depends on: MT-008, MT-005.

### MT-012 - Append-Only Sheet Version Store
- Objective: Implement sheet versions as append-only records.
- Inputs: CKC `SheetVersion` table and `SheetVersionTools.tsx`.
- Work slice: add version records, version creation, parent/source refs, author/tool/source metadata, and current-version projection.
- Outputs: create/list/get version APIs and tests.
- Acceptance: saving or importing creates a new version; prior values remain recoverable.
- Verification: tests for create version, list chronology, current projection, and source evidence refs.
- Depends on: MT-006, MT-008, MT-005.

### MT-013 - Selective Apply And Revert
- Objective: Preserve CKC selective field apply/revert behavior.
- Inputs: `SheetIngestMergeTools.tsx`, `SheetVersionTools.tsx`.
- Work slice: implement preview changes, selectable fields, revert preview, apply selected fields into a new version.
- Outputs: merge/revert APIs and parity fixtures.
- Acceptance: unselected fields remain unchanged; revert creates a new version with provenance instead of mutating history.
- Verification: tests for selective apply, protected field default unselected, revert subset, and unmapped preservation.
- Depends on: MT-012, MT-011.

### MT-014 - Bulk Character Operations
- Objective: Preserve bulk tag/field/export/trash operations with guardrails.
- Inputs: `LibraryView.tsx`, `BulkFieldEditDialog.tsx`, `BulkTagDialog.tsx`, `BatchExportDialog.tsx`, CKC WP-0090.
- Work slice: add backend batch operation contracts and validation for selected character IDs.
- Outputs: bulk operation APIs/events; UI can be projection-only if not in scope.
- Acceptance: multi-select operations validate all targets before mutation and produce one batch receipt.
- Verification: tests for partial invalid target rollback, bulk tags, bulk field preview, batch export manifest, and trash/restore.
- Depends on: MT-006, MT-012, MT-005.

### MT-015 - Media Asset Identity Model
- Objective: Implement media asset identity, provenance, and relation fields.
- Inputs: CKC `ImageAsset` columns and media evidence rows.
- Work slice: add media asset type/table with asset id, character id, artifact ref, content hash, dimensions, source refs, role, deletion state, and revision.
- Outputs: create/list/get media asset APIs.
- Acceptance: assets remain stable across file moves; content hash dedup prevents accidental duplicates.
- Verification: tests for import, duplicate detection, missing artifact ref, and character reassignment.
- Depends on: MT-003, MT-005, MT-007.

### MT-016 - ArtifactStore Media Materialization
- Objective: Route media files through ArtifactStore and manifests instead of random filesystem writes.
- Inputs: Artifact System Foundations; CKC image import and content-hash behavior.
- Work slice: implement media artifact materialization with manifest, hash, size, source path/url/note, and retention flags.
- Outputs: materialize/import function and manifest tests.
- Acceptance: product outputs never land in `.GOV`; manifests can reconstruct source and materialized paths.
- Verification: tests for artifact root, manifest hash, no-space names, and `.GOV` rejection.
- Depends on: MT-015.

### MT-017 - Thumbnail, Proxy, And Photo Skeleton Surface
- Objective: Preserve Photo Studio skeleton surface, thumbnails, and preview/proxy responsibilities.
- Inputs: `WP-1-Photo-Studio-v2`, prior Photo Studio packet if present, Loom poster frame stub.
- Work slice: implement derived thumbnail/proxy asset contract and skeleton projection state; full UI can be later.
- Outputs: thumbnail/proxy metadata, generation status, and retry/error states.
- Acceptance: media assets can show preview state without blocking full media load; missing thumbnail is diagnosable.
- Verification: tests for generated, pending, failed, and missing preview states.
- Depends on: MT-015, MT-016.

### MT-018 - Media Review Metadata
- Objective: Preserve favorites, ratings, `frontpage`, `carousel`, notes, and review status.
- Inputs: CKC `MediaPane.tsx`, `LibraryView.tsx`, `library.js` favorite/rating/frontpage/carousel behavior.
- Work slice: add metadata fields and batch-safe update APIs.
- Outputs: set/get metadata behavior and review projection contract.
- Acceptance: 0..5 ratings clamp correctly; frontpage/carousel tags drive ranking without special hidden state.
- Verification: tests for favorite toggle, rating clamp, frontpage ranking, carousel ranking, and batch update.
- Depends on: MT-015, MT-005.

### MT-019 - Tags, Tag Rules, And Bulk Tagging
- Objective: Preserve tag manager, saved tag rules, and deterministic rule application.
- Inputs: CKC `TagRule`, tag manager APIs, `apply TagRules deterministically by rule_id order`.
- Work slice: implement tag rule storage, enable/disable, match types, emitted tags, and bulk add/remove.
- Outputs: tag APIs and rule application tests.
- Acceptance: derived tags are reproducible; rule changes create rebuild receipts.
- Verification: tests for rule order, add/remove, disabled rule, renamed emit tag, and batch tags.
- Depends on: MT-018.

### MT-020 - Palette, dHash, And Similarity Projections
- Objective: Preserve color palette extraction, dHash computation, and similar-image grouping as derived projections.
- Inputs: CKC `palette.js`, `dhash.js`, `MediaPane.tsx` color/similar UI.
- Work slice: add projection records and rebuild jobs for palette and dHash.
- Outputs: compute/recompute/search APIs and projection receipts.
- Acceptance: missing or invalid hashes do not crash search; similarity threshold is explicit.
- Verification: tests for cached palette, invalid hash repair, distance threshold, and no auto-delete.
- Depends on: MT-015, MT-005.

### MT-021 - AI Tag Suggestion Proposal Boundary
- Objective: Preserve AI tag suggestions as proposals, not automatic truth.
- Inputs: CKC AI tagging job behavior, local model config, `MediaPane.tsx` suggestion controls.
- Work slice: add AI suggestion record type or job output linked to media asset and model/tool receipt.
- Outputs: suggestion list, accept/reject/apply flow contract, and events.
- Acceptance: generated tags are not Raw tags until explicit apply; failed model calls leave diagnosable evidence.
- Verification: tests with fake model output, malformed output, cancel, accept one tag, reject all.
- Depends on: MT-019, Diagnostics local-LLM MTs.

### MT-022 - Sidecar Visibility Matrix
- Objective: Preserve OpenPose and workflow sidecars as artifacts hidden from normal galleries.
- Inputs: CKC `media_role`, `source_image_id`, `openpose_png_path`, Lens ViewMode.
- Work slice: add sidecar role/projection rules and relation lookup.
- Outputs: sidecar visibility matrix and list-by-source API.
- Acceptance: sidecars are searchable by relation but excluded from normal gallery unless explicit projection asks for them.
- Verification: tests for normal gallery exclusion, explicit sidecar projection, source-image relation, and ViewMode immutability.
- Depends on: MT-015, Pose sidecar MTs.

### MT-023 - Filesystem Health Diagnostics
- Objective: Preserve filesystem health checks for missing originals, thumbs, inbox pending, untracked originals, and sidecars.
- Inputs: CKC `filesystemHealthCheck`, taskboard WP-0127.
- Work slice: implement health scan over ArtifactStore refs and imported source refs.
- Outputs: structured health report and EventLedger scan event.
- Acceptance: missing files are reported without auto-resync or deletion.
- Verification: tests for missing original, missing thumb, orphan original, missing sidecar, and healthy state.
- Depends on: MT-016, MT-017, MT-022.

### MT-024 - Deletion Impact Preview And Recoverable Archive
- Objective: Preserve recoverable deletion controls.
- Inputs: CKC `deletionImpactPreview`, `archiveImages`, WP-0135.
- Work slice: implement preview, archive, restore, and soft-delete states for images and sheet versions.
- Outputs: APIs/events for impact preview, archive, restore, and delete rejection.
- Acceptance: destructive actions require preview and are recoverable where CKC made them recoverable.
- Verification: tests for preview graph, archive image, restore image, sheet-version archive/delete, and event audit.
- Depends on: MT-015, MT-012, MT-005.

### MT-025 - Clipboard And URL Image Import
- Objective: Preserve clipboard and URL image import with provenance and capability gates.
- Inputs: CKC `importClipboardImage`, `importFromUrl`, `source_url`, `source_note`.
- Work slice: implement import request contract and safe fetch/materialization path.
- Outputs: import APIs and provenance records.
- Acceptance: URL import stores source URL and retrieval evidence; untrusted URLs are capability-gated and SSRF protected.
- Verification: tests for clipboard import, URL import, duplicate URL, localhost/private IP denial, and failed fetch evidence.
- Depends on: MT-016, MT-004.

### MT-026 - Inbox Folder Scan Import
- Objective: Preserve Inbox folder scan and pending-image import behavior.
- Inputs: CKC `scanInbox`, `inboxDir`, `includeSubdirs`, pending images.
- Work slice: implement configured inbox directory scan with max file bound and duplicate skip.
- Outputs: scan API, scan summary, and imported pending media rows.
- Acceptance: scan never modifies source folders; imported copies get pending/review metadata.
- Verification: tests for include/exclude subdirs, max files, duplicate skip, missing dir, and summary counts.
- Depends on: MT-025, MT-018.

### MT-027 - Source Provenance Fields
- Objective: Preserve source URL/path/note/contact-sheet/task/run refs on media.
- Inputs: CKC `source_url`, `source_path`, `source_note`, source dataset/task/run/contact sheet refs.
- Work slice: add typed provenance fields and validation shared by all import paths.
- Outputs: provenance type, storage fields, and export manifest mapping.
- Acceptance: provenance survives move, export, backup, pending lifecycle, and character reassignment.
- Verification: tests for each source field and round-trip through export/import manifest.
- Depends on: MT-015, MT-025, MT-026.

### MT-028 - Intake Batch Model
- Objective: Implement persistent intake batches.
- Inputs: CKC `IntakeBatch`, `IntakeBatchItem`, `createIntakeBatch`, WP-0124.
- Work slice: add batch/item records, source refs, mode, status, and resume metadata.
- Outputs: create/list/get batch APIs and fixtures.
- Acceptance: batches persist across route switch and restart.
- Verification: tests for create batch, list after restart, empty batch rejection, and source preservation.
- Depends on: MT-015, MT-027.

### MT-029 - Intake Item Lifecycle
- Objective: Implement item states for pending, accepted, rejected, deferred, skipped, and failed.
- Inputs: CKC intake classification and `IngestionRejection`.
- Work slice: add state transitions with idempotent commands and rejection audit rows.
- Outputs: classify item API and transition tests.
- Acceptance: repeat commands are idempotent; invalid transition creates structured error.
- Verification: tests for accept, reject with reason, defer, duplicate accept, invalid state transition.
- Depends on: MT-028, MT-005.

### MT-030 - Loose And Linked Intake Modes
- Objective: Preserve loose profile and character-linked intake behavior.
- Inputs: CKC WP-0124, WP-0125.
- Work slice: implement mode selection and target refs for character, optional sheet version, optional collection.
- Outputs: batch mode fields and validation.
- Acceptance: loose mode does not require character; linked mode validates target character and optional version/collection.
- Verification: tests for loose accept, linked accept, missing target rejection, and version-agnostic default.
- Depends on: MT-028, MT-006, MT-012.

### MT-031 - Intake Classification Apply
- Objective: Apply accept/reject/pending decisions to media records safely.
- Inputs: CKC `classifyIntakeBatch` and pending/inbox lifecycle.
- Work slice: implement batch classification for selected items with tags/notes/status updates.
- Outputs: classification API and receipt.
- Acceptance: accepted images link to chosen target; pending images route to Inbox; rejected rows preserve source and reason only.
- Verification: tests for batch partial classify, rollback on invalid target, pending tag, and rejection audit.
- Depends on: MT-029, MT-030.

### MT-032 - Intake Sheet Version And Collection Links
- Objective: Preserve optional sheet-version and collection linking during intake.
- Inputs: CKC WP-0125, WP-0128.
- Work slice: attach accepted media to sheet version and/or collection without forcing either.
- Outputs: link fields/events and list projections.
- Acceptance: version-agnostic remains default; explicit version and collection refs survive export.
- Verification: tests for no version, valid version, invalid version, collection link, and export manifest.
- Depends on: MT-031, MT-035.

### MT-033 - Pending Inbox Projection
- Objective: Preserve pending Inbox as a review lane.
- Inputs: CKC WP-0126 and Library Inbox lane.
- Work slice: implement query/projection for pending images with counts, selection refs, and source metadata.
- Outputs: pending list API or projection model.
- Acceptance: pending items are visible without hidden route state and can be resumed after restart.
- Verification: tests for pending count, filters, source note display data, and route-independent query.
- Depends on: MT-031.

### MT-034 - Collection Model
- Objective: Implement collections/photo sets with notes, tags, optional character and sheet-version refs.
- Inputs: CKC WP-0128, `Collection`, `CollectionImage`.
- Work slice: add collection records, membership, notes/tags, optional links.
- Outputs: collection CRUD and membership APIs.
- Acceptance: collections preserve per-photo tags and optional character/version context.
- Verification: tests for create, update notes/tags, add/remove images, optional links, and list.
- Depends on: MT-015, MT-006.

### MT-035 - Collection Batch Metadata
- Objective: Preserve batch tag/metadata application from collections.
- Inputs: CKC collection batch tag behavior.
- Work slice: implement collection-scoped media batch operations.
- Outputs: batch metadata API linked to collection selection.
- Acceptance: applying tags through collection preserves existing photo tags unless explicitly removed.
- Verification: tests for add tag, remove tag, favorite/rating batch, invalid image not in collection.
- Depends on: MT-034, MT-019.

### MT-036 - Contact Sheet SVG Manifest
- Objective: Preserve contact sheets as reproducible artifacts.
- Inputs: CKC `ContactSheet`, `createContactSheet`, WP-0129.
- Work slice: create contact sheet record, source image id list, layout metadata, SVG artifact, manifest, hashes.
- Outputs: contact sheet create/list/get APIs and ArtifactStore entries.
- Acceptance: a contact sheet can be regenerated from manifest and source image ids.
- Verification: tests for manifest hash, no-space artifact names, layout metadata, missing source behavior, and collection link.
- Depends on: MT-034, MT-016.

### MT-037 - Raster Contact Sheet Export Hook
- Objective: Preserve planned PNG/JPG contact sheet export without pretending it is done.
- Inputs: CKC WP-0136 PLANNED.
- Work slice: add extension point/status for raster export and block implementation unless activated.
- Outputs: deferred capability flag and acceptance guard.
- Acceptance: SVG/manifest path works; raster export returns planned/deferred status with source refs.
- Verification: test `raster_export_status=DEFERRED_PLANNED` and no fake PNG/JPG output.
- Depends on: MT-036.

### MT-038 - Character Notes And Stories Documents
- Objective: Preserve character-scoped notes/stories/moodboard doc types.
- Inputs: CKC `docType: notes|stories|moodboard`, `upsertDoc`, docs drawer.
- Work slice: implement doc records linked to character with title/body/tags/type and versioning refs.
- Outputs: doc CRUD and list/search projection.
- Acceptance: docs are not stored only as UI state; raw text is preserved.
- Verification: tests for notes, stories, tag filter, update, delete/archive if supported, and search inclusion.
- Depends on: MT-006, MT-005.

### MT-039 - Story Cards And Story Beats
- Objective: Preserve story board cards and StoryBeat records.
- Inputs: CKC `getStoryBoard`, `setStoryBoard`, `StoryBeat`, `listStoryBeats`.
- Work slice: implement ordered story cards and separate story beat records with stable ids.
- Outputs: story board APIs and beat APIs.
- Acceptance: card order and text round-trip; beats can be listed per character.
- Verification: tests for split text into cards, reorder, edit, delete, and beat CRUD.
- Depends on: MT-038.

### MT-040 - Character Scripts
- Objective: Preserve per-character image-sourcing scripts.
- Inputs: CKC `CharacterScript`, `listCharacterScripts`, `getCharacterScript`, add/remove APIs.
- Work slice: implement script records with provenance and usage refs for later workflow registry MTs.
- Outputs: script CRUD and query APIs.
- Acceptance: scripts are linked to character and cannot become hidden executable authority.
- Verification: tests for create/get/list/remove and EventLedger evidence.
- Depends on: MT-006, MT-005.

### MT-041 - Bracket Links And Backlinks
- Objective: Preserve outbound and inbound links between docs, stories, moodboards, images, and characters.
- Inputs: CKC bracket link extraction, notes/stories/moodboard backlinks.
- Work slice: implement link extraction and backlink projection from text-bearing records.
- Outputs: link projection records and query APIs.
- Acceptance: links are derived and rebuildable; raw source text is unchanged.
- Verification: tests for outbound link, backlink, deleted target, moodboard text link, and rebuild receipt.
- Depends on: MT-038, MT-039, MT-042.

### MT-042 - Moodboard JSON Schema And Layer Model
- Objective: Preserve full moodboard structure, not just attachment behavior.
- Inputs: CKC `MoodboardDoc`, `MoodboardCanvas.tsx`.
- Work slice: implement moodboard state schema with images, text, shapes, connectors, folders, layers, guides, hidden/locked flags, style fields, and history hooks.
- Outputs: moodboard schema/types and validation.
- Acceptance: a saved moodboard round-trips all supported item kinds and layer flags.
- Verification: fixture round-trip with image, text, shape, connector, folder, guide, locked/hidden.
- Depends on: MT-038, MT-015.

### MT-043 - Moodboard Operations And Export Hooks
- Objective: Preserve moodboard operations that affect data model and export.
- Inputs: CKC copy/paste, arrange, tags, search, PNG/PDF export functions.
- Work slice: add data operations for selection, item tags, ordering, hidden/locked toggles, and export job request contracts.
- Outputs: operation APIs and export request types.
- Acceptance: export hooks produce manifests/receipts and never write product outputs under `.GOV`.
- Verification: tests for tag selected items, reorder layer, hidden toggle, PNG export request, PDF export request.
- Depends on: MT-042, MT-016.

### MT-044 - Character Relations And Relationship Map Projection
- Objective: Preserve character relationships and future relationship-map projection.
- Inputs: CKC `CharacterRelation`, preload/main handlers, automation manual relationship group.
- Work slice: implement relation CRUD with source/target character ids, relation type, notes, and derived graph projection.
- Outputs: relation APIs and graph projection contract.
- Acceptance: relations validate both endpoints and can be queried per character and as a graph.
- Verification: tests for create/update/delete/list, invalid target rejection, graph projection, and export inclusion.
- Depends on: MT-006, MT-005.

### MT-045 - Global Search With Snippets And Jump Targets
- Objective: Preserve CKC global search across sheets, notes, images, moodboards, and docs.
- Inputs: CKC `globalSearch`, `GlobalSearchModal.tsx`, saved search rows.
- Work slice: implement search index/projection through Handshake search architecture, not SQLite FTS.
- Outputs: search query API with snippets and jump target refs.
- Acceptance: results include kind, stable id, snippet, and route/jump target for every indexed domain.
- Verification: tests for sheet hit, doc hit, image metadata hit, moodboard text hit, and no SQLite usage.
- Depends on: MT-038, MT-042, MT-015, MT-012.

### MT-046 - Lens ExtractionTier And ViewMode Filters
- Objective: Preserve LensExtractionTier and LensViewMode rules in Core search/projection.
- Inputs: Lens Extraction Tier stub; Lens ViewMode stub.
- Work slice: add requested/effective extraction tier and ViewMode filter metadata to query/search projection.
- Outputs: query plan fields and validation.
- Acceptance: Tier1 defaults; Tier0/Tier2 override is trace-visible; SFW ViewMode hard-drops non-sfw projected items without mutating raw artifacts.
- Verification: tests for default tier, invalid tier rejection, explicit tier trace, SFW projection, NSFW default.
- Depends on: MT-045.

### MT-047 - Saved Searches And Retrieval Projections
- Objective: Preserve saved searches as durable reusable filters.
- Inputs: CKC `SavedSearch`, global filters, tag/color/rating filters.
- Work slice: implement saved search CRUD with filter schema and owner refs.
- Outputs: saved search APIs and query execution hook.
- Acceptance: saved search can reproduce query including tags, exclusions, ratings, favorite, color, scope, and ViewMode.
- Verification: tests for create/list/run/update/delete and invalid filter schema.
- Depends on: MT-045, MT-046.

### MT-048 - Web Portfolio Export Contract
- Objective: Preserve CKC web portfolio export as a portable ArtifactStore-backed export contract.
- Inputs: CKC `exportWebPortfolio`, portfolio export examples, ArtifactStore materialization rules.
- Work slice: implement or specify only the web portfolio export request/result schema, manifest fields, selected character/media inputs, generated index/assets refs, checksums, and no-space output naming.
- Outputs: web portfolio export request/result schemas, manifest schema, and focused manifest tests.
- Acceptance: web portfolio export includes source ids, checksums, safe asset refs, README/usage where required, no `.GOV` output, and no machine-local absolute paths.
- Verification: tests for portfolio export manifest, checksum fields, no-space paths, and `.GOV` output rejection.
- Depends on: MT-014, MT-015, MT-038, MT-044.

### MT-049 - Backup Manifest And Version Guards
- Objective: Preserve backup version traceability and restore refusal rules.
- Inputs: CKC `backup.js`, WP-0106 backup version/cursor fields.
- Work slice: implement backup manifest with app/spec/schema versions, checksums, source refs, and newer-app restore refusal.
- Outputs: backup/export manifest and restore preflight.
- Acceptance: restore preflight refuses newer incompatible backups and explains why.
- Verification: tests for backup manifest fields, checksum mismatch, newer version refusal, valid restore preflight.
- Depends on: MT-016.

### MT-050 - Reset Modes And Orphan Adoption
- Objective: Preserve installer/reset/orphan adoption data behavior.
- Inputs: CKC `resetModes.js`, WP-0105, `adoptOrphanImages`.
- Work slice: implement data-root reset contracts for preferences reset, full reset preserving original images where applicable, orphan manifest, and adoption.
- Outputs: reset request/result schemas and orphan adoption APIs.
- Acceptance: full reset preserves configured original media bytes and writes an orphan manifest; adoption is explicit and receipt-backed.
- Verification: tests for light reset, full reset manifest, adopt orphan, invalid manifest, and EventLedger records.
- Depends on: MT-016, MT-049.

### MT-051 - Build, Package, Release, And Data Root Lessons
- Objective: Preserve CKC package/release/data-root lessons as Core acceptance constraints.
- Inputs: CKC WP-0001, WP-0008, WP-0017, WP-0019, WP-0020, WP-0044, WP-0086, package scripts.
- Work slice: add data-root and export-root rules consumed by release/build diagnostics.
- Outputs: config docs/contracts for portable roots, dev/release artifact roots, and staged build outputs.
- Acceptance: product data and generated artifacts are relocatable; build artifacts remain outside product source and carry build id/version metadata.
- Verification: static checks for drive-letter paths, relative asset-root docs, artifact-root config, and no repo `dist/`.
- Depends on: MT-016, Diagnostics build MTs.

### MT-052 - Character And Sheet Manual Source Rows
- Objective: Provide Diagnostics with no-context manual rows for character identity and sheet workflows only.
- Inputs: MT-006 through MT-015 and protected-field decisions from MT-008.
- Work slice: write manual source rows for create character, inspect identity, import sheet text, parse template fields, edit non-protected fields, create sheet version, compare versions, and revert version.
- Outputs: manual rows with command/action ids, required inputs, outputs, safety notes, common errors, recovery steps, and evidence paths for character/sheet actions.
- Acceptance: a no-context model can operate character/sheet workflows without reading implementation code or old CKC docs.
- Verification: manual-row coverage check for character id, template parse, protected fields, version history, merge/apply, and revert.
- Depends on: MT-006 through MT-015.

### MT-053 - Media And Intake Manual Source Rows
- Objective: Provide Diagnostics with no-context manual rows for media library and intake workflows only.
- Inputs: MT-018 through MT-032 and downloader/ASR carry-forward rows.
- Work slice: write manual source rows for import media, inspect media provenance, generate thumbnail/proxy, hide sidecars from gallery, create intake batch, accept/reject/defer item, resume intake, and inspect source URL/path/note provenance.
- Outputs: manual rows with action ids, params, outputs, failure modes, recovery steps, and evidence artifacts for media/intake actions.
- Acceptance: manual rows explain missing-file handling, dedup behavior, sidecar visibility, and review lifecycle without cross-reading the stub.
- Verification: manual-row coverage check for media identity, artifact metadata, thumbnails, sidecars, intake status, provenance, and resume.
- Depends on: MT-018 through MT-032.

### MT-054 - Collection And Contact Sheet Manual Source Rows
- Objective: Provide Diagnostics with no-context manual rows for collection and contact-sheet workflows only.
- Inputs: MT-033 and MT-034.
- Work slice: write manual rows for create/update collection, add/remove media from collection, edit collection notes/tags, create contact sheet, inspect contact-sheet layout metadata, and locate source images.
- Outputs: manual rows with action ids, inputs, outputs, safety notes, failure modes, recovery steps, and evidence paths for collection/contact-sheet actions.
- Acceptance: a no-context model can operate collection/contact-sheet workflows without reading implementation code.
- Verification: manual-row coverage check for collections, notes/tags, contact-sheet metadata, source image hashes, and deferred raster export status.
- Depends on: MT-033, MT-034.

### MT-055 - Docs Stories And Scripts Manual Source Rows
- Objective: Provide Diagnostics with no-context manual rows for document, story, beat, and script workflows only.
- Inputs: MT-035 through MT-040.
- Work slice: write manual rows for create/edit note, create/edit story, add story beat, edit script, preserve original text, inspect backlinks, and inspect outbound links.
- Outputs: manual rows with action ids, inputs, outputs, safety notes, failure modes, recovery steps, and evidence paths for text/story/script actions.
- Acceptance: a no-context model can operate narrative text workflows while preserving original text bytes and links.
- Verification: manual-row coverage check for notes, stories, beats, scripts, byte preservation, backlinks, and outbound links.
- Depends on: MT-035 through MT-041.

### MT-056 - Moodboard Manual Source Rows
- Objective: Provide Diagnostics with no-context manual rows for moodboard workflows only.
- Inputs: MT-042 and MT-043.
- Work slice: write manual rows for create moodboard, add node, move node, add layer/folder, add comment, link moodboard item to character/media/doc, and export moodboard manifest.
- Outputs: manual rows with action ids, inputs, outputs, safety notes, failure modes, recovery steps, and evidence paths for moodboard actions.
- Acceptance: a no-context model can operate moodboard workflows without confusing canvas data with raster export.
- Verification: manual-row coverage check for schema fields, nodes, layers/folders, comments, links, and export hook.
- Depends on: MT-042, MT-043.

### MT-057 - Relationship Manual Source Rows
- Objective: Provide Diagnostics with no-context manual rows for character relationship workflows only.
- Inputs: MT-044.
- Work slice: write manual rows for create relationship row, edit relation type, link source character, link target character, inspect relation provenance, and inspect relationship-map projection inputs.
- Outputs: manual rows with action ids, inputs, outputs, safety notes, failure modes, recovery steps, and evidence paths for relationship actions.
- Acceptance: relationship rows and future map projections are documented as data/projection boundaries, not a finished graph UI.
- Verification: manual-row coverage check for relation fields, source/target references, provenance, and projection inputs.
- Depends on: MT-044.

### MT-058 - Search Tag Palette And Similarity Manual Source Rows
- Objective: Provide Diagnostics with no-context manual rows for retrieval and visual-filter workflows only.
- Inputs: MT-045 through MT-047.
- Work slice: write manual rows for run search, open snippet jump target, create/update tag, run saved search, filter by palette/color, inspect dHash similarity group, and review/apply AI tag suggestions.
- Outputs: manual rows with action ids, inputs, outputs, safety notes, failure modes, recovery steps, and evidence paths for retrieval/filter actions.
- Acceptance: search and tag rows never imply SQLite FTS, and AI suggestions are separate from applied tags.
- Verification: manual-row coverage check for snippets, jump targets, tags, saved searches, palette filters, dHash similarity, and suggestion/apply separation.
- Depends on: MT-045 through MT-047.

### MT-059 - Web Portfolio Export Manual Source Rows
- Objective: Provide Diagnostics with no-context manual rows for web portfolio export only.
- Inputs: MT-048.
- Work slice: write manual rows for starting a web portfolio export, selecting portfolio inputs, inspecting the export manifest, locating generated index/assets refs, and recovering from invalid output roots.
- Outputs: manual rows with action ids, inputs, outputs, safety notes, failure modes, recovery steps, and artifact/evidence paths for web portfolio export.
- Acceptance: a no-context model can run and troubleshoot web portfolio export without guessing share-pack or LLM-pack behavior.
- Verification: manual-row coverage check for portfolio export request, manifest, checksums, output root rejection, and artifact refs.
- Depends on: MT-048.

### MT-060 - Reset Recovery And Orphan Adoption Manual Source Rows
- Objective: Provide Diagnostics with no-context manual rows for reset, recovery, and orphan-adoption workflows only.
- Inputs: MT-050 and MT-051.
- Work slice: write manual rows for light reset, full reset with original media preservation, orphan manifest inspection, orphan adoption, and recovery from invalid manifests or non-portable roots.
- Outputs: manual rows with action ids, inputs, outputs, safety notes, failure modes, recovery steps, and artifact/evidence paths for reset/recovery actions.
- Acceptance: a no-context model can run data-preserving recovery workflows without guessing which media is preserved or orphaned.
- Verification: manual-row coverage check for reset receipt, orphan manifest, adoption receipt, preserved original media, invalid manifest error, and root portability.
- Depends on: MT-050, MT-051.

### MT-061 - Character And Sheet Fixture Corpus
- Objective: Create portable non-SQLite fixtures for character identity and sheet parsing/versioning.
- Inputs: CKC character tests, sheet parser examples, MT-006 through MT-015.
- Work slice: add fixture files/data builders for public/internal character ids, aliases, template fields, protected fields, block lists, parsed sheet sections, append-only versions, merge/apply proposals, and revert cases.
- Outputs: character/sheet fixture family under product test fixtures or official packet evidence folder.
- Acceptance: fixtures contain no SQLite DB files, no CKC runtime namespace, and no machine-local absolute paths.
- Verification: fixture linter plus focused parser/version tests consuming the fixture family.
- Depends on: MT-001, MT-004, MT-006 through MT-015.

### MT-062 - Media And Intake Fixture Corpus
- Objective: Create portable non-SQLite fixtures for media identity and intake review behavior.
- Inputs: CKC media tests, intake examples, MT-018 through MT-032.
- Work slice: add fixture files/data builders for media source references, content hashes, metadata, thumbnails/proxy metadata, sidecar references, source URL/path/note provenance, intake batches, accept/reject/defer states, and resume cases.
- Outputs: media/intake fixture family under product test fixtures or official packet evidence folder.
- Acceptance: fixtures can be copied between machines without broken absolute paths and without exposing sidecars as gallery items.
- Verification: fixture linter plus focused media/import/intake tests consuming the fixture family.
- Depends on: MT-001, MT-004, MT-018 through MT-032.

### MT-063 - Collection And Contact Sheet Fixture Corpus
- Objective: Create portable fixtures for collections and contact-sheet artifacts only.
- Inputs: MT-033 and MT-034.
- Work slice: add fixture files/data builders for collection ids, collection notes, tags, linked media ids, optional character links, contact-sheet layout metadata, source image hashes, and deferred raster export references.
- Outputs: collection/contact-sheet fixture family.
- Acceptance: fixtures prove collection/contact-sheet metadata can exist without raster export and without CKC namespace.
- Verification: fixture linter plus focused collection/contact-sheet tests consuming the fixture family.
- Depends on: MT-033, MT-034.

### MT-064 - Docs, Moodboard, And Relations Fixture Corpus
- Objective: Create portable fixtures for narrative docs, moodboards, and relationship rows only.
- Inputs: MT-035 through MT-040.
- Work slice: add fixtures for notes, stories, story beats, scripts, backlinks, outbound links, moodboard nodes/layers/folders/comments, character references, relationship rows, and relationship-map projection inputs.
- Outputs: docs/moodboard/relations fixture family.
- Acceptance: fixtures preserve original text bytes and links/backlinks while staying product-namespace neutral.
- Verification: fixture linter plus focused docs/moodboard/relation tests consuming the fixture family.
- Depends on: MT-035 through MT-040.

### MT-065 - Search, Tag, Palette, And Similarity Fixture Corpus
- Objective: Create portable fixtures for retrieval and visual-filter projections only.
- Inputs: MT-041 through MT-045.
- Work slice: add fixtures for search documents, snippets, jump targets, tags, saved searches, palette/color metadata, dHash values, similarity groups, and AI tag suggestion proposal/apply examples.
- Outputs: search/tag/palette/similarity fixture family.
- Acceptance: fixtures do not assume SQLite FTS and clearly separate suggestions from applied tags.
- Verification: fixture linter plus focused search/tag/similarity tests consuming the fixture family.
- Depends on: MT-041 through MT-045.

### MT-066 - Reset And Orphan Fixture Corpus
- Objective: Create portable fixtures for reset and orphan-adoption behavior only.
- Inputs: MT-050 and MT-051.
- Work slice: add fixtures for light reset request/result, full reset request/result, preserved original media refs, orphan manifests, invalid orphan manifests, and adoption receipts.
- Outputs: reset/orphan fixture family.
- Acceptance: fixtures include checksum examples and relocatable artifact roots but no drive-letter paths.
- Verification: fixture linter plus focused reset/orphan tests consuming the fixture family.
- Depends on: MT-050, MT-051.

### MT-067 - Core Integration Smoke Path
- Objective: Define and test one end-to-end Core path.
- Inputs: completed Core APIs from earlier MTs.
- Work slice: create a smoke test that creates character -> sheet version -> import image -> intake accept -> collection -> contact sheet -> search -> export manifest.
- Outputs: integration test and evidence receipt.
- Acceptance: the path runs without GUI and proves source ids and events connect.
- Verification: run the focused integration test; inspect events/artifact manifests.
- Depends on: MT-006, MT-012, MT-015, MT-031, MT-034, MT-036, MT-045, MT-048.

### MT-068 - Core Red-Team Data Mutation Guards
- Objective: Turn character/sheet data-loss risks into enforceable negative checks.
- Inputs: repaired stub risks for silent overwrite, source loss, protected-field mutation, and version flattening.
- Work slice: add negative tests/checks that attempt to overwrite original sheet text, mutate protected fields, drop source provenance, flatten append-only versions, and apply stale merge proposals.
- Outputs: focused red-team checks for character/sheet mutation guards.
- Acceptance: each negative case fails when the guard is removed and reports the violated invariant.
- Verification: run focused red-team checks for source preservation, protected fields, version history, and stale proposal rejection.
- Depends on: MT-006 through MT-015, MT-004.

### MT-069 - Core Red-Team Artifact Path And Runtime Guards
- Objective: Turn artifact-path and runtime-assumption risks into enforceable negative checks.
- Inputs: repaired stub risks for `.GOV` product output, SQLite, local path leak, CKC namespace, and non-portable roots.
- Work slice: add checks that reject product outputs under `.GOV`, SQLite files/adapters/fixtures, drive-letter or profile paths, CKC runtime namespace strings, and unconfigured artifact roots.
- Outputs: focused red-team checks for artifact/runtime portability.
- Acceptance: each rejected runtime assumption has a direct failing check and remediation text.
- Verification: run focused red-team checks for output path, SQLite, absolute path, namespace, and artifact-root policy.
- Depends on: MT-016, MT-046 through MT-051, MT-004.

### MT-070 - Core Red-Team Review And Projection Guards
- Objective: Turn review-status and projection risks into enforceable negative checks.
- Inputs: repaired stub risks for sidecar gallery clutter, review-status flattening, provenance loss, and AI suggestion/apply confusion.
- Work slice: add checks that sidecars stay hidden from normal galleries, DONE/REVIEW/BLOCKED/PLANNED statuses do not flatten, media provenance survives import/review/export, and AI tag suggestions cannot appear as applied tags without an explicit receipt.
- Outputs: focused red-team checks for review/projection boundaries.
- Acceptance: each projection/review risk has a failing negative case when the relevant guard is removed.
- Verification: run focused red-team checks for sidecar visibility, status maturity, provenance preservation, and AI suggestion/apply separation.
- Depends on: MT-019, MT-022, MT-031, MT-032, MT-045.

### MT-071 - Share Pack Export Contract
- Objective: Preserve CKC share-pack export as a portable safe-subset export contract.
- Inputs: CKC `exportSharePack`, share-pack examples, safe subset rules, ArtifactStore materialization rules.
- Work slice: implement or specify only the share-pack export request/result schema, manifest fields, subset selector, included media/docs metadata, checksums, and README/usage artifact.
- Outputs: share-pack export request/result schemas, manifest schema, and focused manifest tests.
- Acceptance: share packs include only allowed subset data, preserve source ids/checksums, include usage notes where required, and reject `.GOV` output.
- Verification: tests for share-pack subset selection, manifest fields, checksums, README/usage refs, and no-space output paths.
- Depends on: MT-014, MT-015, MT-038, MT-044.

### MT-072 - LLM Evidence Pack Export Contract
- Objective: Preserve CKC spin-off/LLM pack format as a strict model-consumable export contract.
- Inputs: CKC spin-off/LLM pack examples, no-context model evidence needs, ArtifactStore materialization rules.
- Work slice: implement or specify only the LLM evidence pack request/result schema, required file list, manifest fields, source anchors, redaction/safe-subset flags, and strict format validation.
- Outputs: LLM evidence pack export schemas, strict-format validator, and focused tests.
- Acceptance: LLM pack output is deterministic, source-linked, checksum-backed, no-space named, and invalid when required evidence files or anchors are missing.
- Verification: tests for strict format success, missing required file failure, missing source anchor failure, checksum fields, and `.GOV` output rejection.
- Depends on: MT-014, MT-015, MT-038, MT-044.

### MT-073 - Share Pack Export Manual Source Rows
- Objective: Provide Diagnostics with no-context manual rows for share-pack export only.
- Inputs: MT-071.
- Work slice: write manual rows for starting share-pack export, choosing allowed subset inputs, inspecting the manifest, reading usage notes, and recovering from invalid subset/output-root errors.
- Outputs: manual rows with action ids, inputs, outputs, safety notes, failure modes, recovery steps, and artifact/evidence paths for share-pack export.
- Acceptance: a no-context model can run and troubleshoot share-pack export without confusing it with web portfolio or LLM pack export.
- Verification: manual-row coverage check for share-pack request, subset rules, manifest, usage notes, output root rejection, and artifact refs.
- Depends on: MT-071.

### MT-074 - LLM Evidence Pack Manual Source Rows
- Objective: Provide Diagnostics with no-context manual rows for LLM evidence pack export only.
- Inputs: MT-072.
- Work slice: write manual rows for starting LLM evidence pack export, choosing evidence scope, inspecting strict-format files, validating source anchors, and recovering from missing required evidence.
- Outputs: manual rows with action ids, inputs, outputs, safety notes, failure modes, recovery steps, and artifact/evidence paths for LLM evidence packs.
- Acceptance: a no-context model can generate and validate an LLM evidence pack without inferring format from source code.
- Verification: manual-row coverage check for evidence-pack request, required files, source anchors, strict validator, failure modes, and artifact refs.
- Depends on: MT-072.

### MT-075 - Backup Manual Source Rows
- Objective: Provide Diagnostics with no-context manual rows for backup and restore preflight only.
- Inputs: MT-049.
- Work slice: write manual rows for creating a backup, inspecting backup metadata, running restore preflight, handling checksum mismatch, and handling newer incompatible backup refusal.
- Outputs: manual rows with action ids, inputs, outputs, safety notes, failure modes, recovery steps, and artifact/evidence paths for backup workflows.
- Acceptance: a no-context model can explain backup traceability and restore refusal without reading implementation code.
- Verification: manual-row coverage check for backup creation, metadata, checksum mismatch, newer version refusal, valid preflight, and restore receipt.
- Depends on: MT-049.

### MT-076 - Web Portfolio Export Fixture Corpus
- Objective: Create portable fixtures for web portfolio export only.
- Inputs: MT-048.
- Work slice: add fixtures for portfolio export request, selected character/media ids, manifest, generated index/assets refs, checksums, and invalid output root.
- Outputs: web portfolio export fixture family.
- Acceptance: fixtures are relocatable, no-space named, and contain no `.GOV` product output paths.
- Verification: fixture linter plus focused web portfolio export tests consuming the fixture family.
- Depends on: MT-048.

### MT-077 - Share Pack Export Fixture Corpus
- Objective: Create portable fixtures for share-pack export only.
- Inputs: MT-071.
- Work slice: add fixtures for share-pack request, allowed subset selector, included media/docs metadata, manifest, usage notes, checksums, and rejected disallowed item.
- Outputs: share-pack export fixture family.
- Acceptance: fixtures prove subset safety and manifest completeness without CKC runtime namespace.
- Verification: fixture linter plus focused share-pack export tests consuming the fixture family.
- Depends on: MT-071.

### MT-078 - LLM Evidence Pack Fixture Corpus
- Objective: Create portable fixtures for LLM evidence pack export only.
- Inputs: MT-072.
- Work slice: add fixtures for evidence-pack request, required file list, manifest, source anchors, redaction flags, checksum fields, and missing-required-file failure.
- Outputs: LLM evidence pack fixture family.
- Acceptance: fixtures prove strict format validation and source-anchor requirements without SQLite or machine-local paths.
- Verification: fixture linter plus focused LLM evidence pack tests consuming the fixture family.
- Depends on: MT-072.

### MT-079 - Backup Fixture Corpus
- Objective: Create portable fixtures for backup and restore-preflight behavior only.
- Inputs: MT-049.
- Work slice: add fixtures for backup metadata, app/spec/schema version fields, checksums, source refs, valid preflight, checksum mismatch, and newer incompatible backup refusal.
- Outputs: backup fixture family.
- Acceptance: fixtures are relocatable and prove version/refusal rules without touching real operator data.
- Verification: fixture linter plus focused backup/restore-preflight tests consuming the fixture family.
- Depends on: MT-049.

### MT-080 - Core Activation Refinement Closure
- Objective: Convert the repaired stub plus this MT suite into official activation-ready refinement content.
- Inputs: completed source matrix, anchor map, MT suite, red-team controls.
- Work slice: create the official refinement section for Core/Data scope, non-goals, dependencies, risks, acceptance, and MT plan.
- Outputs: refinement draft ready for operator signature.
- Acceptance: no unresolved source row is hidden; no MT depends on an unverified product anchor without saying so.
- Verification: packet/refinement contract check selected at activation.
- Depends on: MT-001 through MT-079.

</topic>
