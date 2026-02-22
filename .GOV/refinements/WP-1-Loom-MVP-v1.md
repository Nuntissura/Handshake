## TECHNICAL_REFINEMENT (MASTER SPEC)

Requirements (HARD):
- This block is REQUIRED for any packet with Status: Ready for Dev / In Progress.
- For EACH SPEC_ANCHOR, include an excerpt window (start/end lines) AND a context token that must appear within that window in the current SPEC_TARGET_RESOLVED spec file.
- Matching rule: context match only (token-in-window), not exact content match.
- Even when ENRICHMENT_NEEDED=NO, you MUST include REASON_NO_ENRICHMENT and SPEC_EXCERPTS for every anchor.
- If ENRICHMENT_NEEDED=YES, you MUST include the full Proposed Spec Enrichment text (verbatim Markdown) that could be copy-pasted into the Master Spec.
- Keep this file ASCII-only. Non-ASCII characters must be written as \\uXXXX escapes inside the excerpt block.
- This file is the Technical Refinement Block required by ORCHESTRATOR_PROTOCOL Part 2.5.2.

### METADATA
- WP_ID: WP-1-Loom-MVP-v1
- CREATED_AT: 2026-02-22T15:15:34.119Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.134.md
- SPEC_TARGET_SHA1: b846f04093f1bd6fae885876affc99a21065ec95
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja220220261648
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Loom-MVP-v1

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- No blocking gaps found for Loom MVP: spec defines LoomBlock schema, LoomEdge semantics for @mentions/#tags/backlinks, import pipeline with SHA-256 dedup, cache tiers + mechanical thumbnail job requirement, Tier-1 search API requirements, and FR-EVT-LOOM event shapes.
- Non-blocking note: older stub metadata references v02.131, but current spec target is v02.134; anchors below use v02.134.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- FR-EVT-LOOM-001: emit on LoomBlock creation.
- FR-EVT-LOOM-002: emit on LoomBlock metadata update (fields_changed[], updated_by).
- FR-EVT-LOOM-003: emit on LoomBlock deletion.
- FR-EVT-LOOM-004: emit on Loom edge creation (mention/tag/sub_tag/parent/ai_suggested).
- FR-EVT-LOOM-005: emit on Loom edge deletion.
- FR-EVT-LOOM-006: emit on dedup hit during import (content_hash, existing_block_id, attempted_filename).
- FR-EVT-LOOM-007: emit when Tier-1 preview is generated (preview_tier, format, duration_ms).
- FR-EVT-LOOM-011: emit when a Loom view is queried/opened (view_type, filter_count, result_count, duration_ms).
- FR-EVT-LOOM-012: emit when Loom search is executed (query_length, tier_used, result_count, duration_ms).

### RED_TEAM_ADVISORY (security failure modes)
- File import attack surface: path traversal, symlink loops, and unexpected device files during folder import; must enforce safe, bounded traversal and treat imported paths as untrusted inputs.
- Resource exhaustion: large folder imports can create DoS via hashing + preview generation + indexing; need cancellation, rate limits, and bounded concurrency on background jobs.
- Inline token integrity: mentions/tags must remain UUID-stable across edits; prevent forged UUID injection or edge corruption via malformed editor payloads.
- Graph spam: auto-creating LoomBlocks for missing @mentions/#tags can explode entity counts if driven by paste/automation; require bounded creation per edit/import operation.
- Preview generation: external tooling (if any) must be capability-gated and sandboxed; outputs must be artifacts with provenance, never arbitrary filesystem writes.

### PRIMITIVES (traits/structs/enums)
- LoomBlock, LoomBlockContentType, LoomBlockDerived, PreviewStatus (entity + derived fields: thumbnail_asset_id, proxy_asset_id, preview_status, full_text_index).
- LoomEdge, LoomEdgeType, LoomEdge.source_anchor (document offsets for inline tokens).
- Import pipeline primitive: content_hash (SHA-256) + workspace-scoped dedup behavior.
- Search primitive: backend-agnostic `search_loom_blocks(query, filters) -> Vec<LoomBlockSearchResult>` with Tier-1 (SQLite FTS5) as the Phase-1 baseline.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [ ] PASS
- Explicitly named: [ ] PASS
- Specific: [ ] PASS
- Measurable acceptance criteria: [ ] PASS
- No ambiguity: [ ] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Master Spec Main Body defines LoomBlock schema + requirements, LoomEdge semantics for mentions/tags/backlinks, Loom import pipeline (SHA-256 dedup + background preview job), Loom view projections + filtering requirements, Loom search API requirements, and the required FR-EVT-LOOM event shapes. This is sufficient to implement and validate Loom MVP without adding new normative text.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: N/A

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Loom MVP requirements are already explicit and testable via the anchored LM-* requirements and FR-EVT-LOOM telemetry contracts.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.134.md 2.2.1.14 LoomBlock Entity (Heaper-style Unit of Meaning) [ADD v02.130]
- CONTEXT_START_LINE: 1254
- CONTEXT_END_LINE: 1347
- CONTEXT_TOKEN: #### 2.2.1.14 LoomBlock Entity (Heaper-style Unit of Meaning) [ADD v02.130]
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 2.2.1.14 LoomBlock Entity (Heaper-style Unit of Meaning) [ADD v02.130]
  
  **Why**  
  Handshake\\u00E2\\u20AC\\u2122s existing **Block** entity (\\u00C2\\u00A72.2.1) is a unit of *Document* content (paragraph, heading, code block). Loom (derived from Heaper patterns) needs a broader **unit of meaning** that can bind a binary **Asset** and a rich\\u00E2\\u20AC\\u2018text **Document** as one object while keeping Handshake\\u00E2\\u20AC\\u2122s **Raw/Derived/Display** discipline (\\u00C2\\u00A72.2.2).
  
  **What**  
  `LoomBlock` is a first-class workspace entity that represents one \\u00E2\\u20AC\\u0153unit of meaning\\u00E2\\u20AC\\u009D in the Loom surface (\\u00C2\\u00A710.12). It may contain:
  - rich text only (NOTE)
  - file only (FILE)
  - file + rich text context (ANNOTATED_FILE)
  - a tag hub that itself has content and backlinks (TAG_HUB)
  - a journal entry (JOURNAL)
  
  ```typescript
  interface LoomBlock {
    block_id: UUID;
    workspace_id: UUID;
  
    // Content
    content_type: LoomBlockContentType;
    document_id?: UUID;          // Points to a CRDT Document for rich text (nullable)
    asset_id?: UUID;             // Points to an Asset for file content (nullable)
  
    // Identity
    title?: string;              // User-assigned display name (independent of filename)
    original_filename?: string;  // Preserved from import (never used for identity)
    content_hash?: SHA256Hex;    // For dedup; inherited from Asset if present
  
    // Organization (RawContent \\u00E2\\u20AC\\u201D user-authored)
    pinned: boolean;
    journal_date?: DateString;   // If this is a daily/weekly note (ISO date)
  
    // Timestamps
    created_at: Timestamp;
    updated_at: Timestamp;
    imported_at?: Timestamp;     // When file was added to loom
  
    // Derived metadata (DerivedContent \\u00E2\\u20AC\\u201D regenerable)
    derived: LoomBlockDerived;
  }
  
  enum LoomBlockContentType {
    NOTE = 'note',                    // Rich text only (no file)
    FILE = 'file',                    // File reference only (no annotations yet)
    ANNOTATED_FILE = 'annotated_file',// File + rich text context
    TAG_HUB = 'tag_hub',              // Tag that holds content and sub-tags
    JOURNAL = 'journal',              // Daily/weekly note
  }
  
  interface LoomBlockDerived {
    // Search
    full_text_index?: string;     // Concatenated searchable text
    embedding_id?: UUID;          // Vector embedding reference
  
    // AI-generated (follows \\u00C2\\u00A72.2.3.2 AIGeneratedMetadata pattern)
    auto_tags?: string[];
    auto_caption?: string;
    quality_score?: number;
  
    // Link metrics (materialized; rebuildable)
    backlink_count: number;
    mention_count: number;
    tag_count: number;
  
    // Media (if asset_id present)
    thumbnail_asset_id?: UUID;
    proxy_asset_id?: UUID;
    preview_status: PreviewStatus;
  
    generated_by?: {
      model: string;
      version: string;
      timestamp: Timestamp;
    };
  }
  
  enum PreviewStatus {
    NONE = 'none',
    PENDING = 'pending',
    GENERATED = 'generated',
    FAILED = 'failed',
  }
  ```
  
  **Normative requirements**
  
  - **[LM-BLOCK-001]** LoomBlock MUST be a first-class workspace entity with a global UUID, accessible via the unified node schema (\\u00C2\\u00A72.2.1.1).
  - **[LM-BLOCK-002]** LoomBlock MUST NOT duplicate data stored in Document or Asset entities. The `document_id` and `asset_id` fields are references, not copies. The LoomBlock is a lightweight wrapper that binds a file to its context.
  - **[LM-BLOCK-003]** When a LoomBlock has both `document_id` and `asset_id`, the rich-text Document is the user\\u00E2\\u20AC\\u2122s context/annotation layer, and the Asset is the canonical file. Both are RawContent. Neither may be silently modified by AI (\\u00C2\\u00A72.2.2.1 rules apply).
  - **[LM-BLOCK-004]** `LoomBlock.title` is independent of `Asset.original_filename`. Users MAY rename a LoomBlock without affecting the underlying file. Identity is about meaning, not filesystem naming.
  - **[LM-BLOCK-005]** `LoomBlock.derived` fields are DerivedContent per \\u00C2\\u00A72.2.2.2 rules: versioned, attributable, prunable, and regenerable.
  - **[LM-BLOCK-006]** LoomBlock creation MUST be logged as a Flight Recorder event (see \\u00C2\\u00A711.5.12).
  
  ---
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.134.md 2.3.7.1 Loom Relational Edges (Mentions, Tags, Backlinks) [ADD v02.130]
- CONTEXT_START_LINE: 2458
- CONTEXT_END_LINE: 2513
- CONTEXT_TOKEN: #### 2.3.7.1 Loom Relational Edges (Mentions, Tags, Backlinks) [ADD v02.130]
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 2.3.7.1 Loom Relational Edges (Mentions, Tags, Backlinks) [ADD v02.130]
  
  Loom introduces a **block-level relational layer** (derived from Heaper patterns) implemented as Knowledge Graph edges with explicit `edge_type` values, plus an anchor back into the source document for inline tokens.
  
  ```typescript
  interface LoomEdge {
    edge_id: UUID;
    source_block_id: UUID;
    target_block_id: UUID;
    edge_type: LoomEdgeType;
  
    // Provenance
    created_by: 'user' | 'ai';
    created_at: Timestamp;
    crdt_site_id?: string;       // CRDT participant who created this edge
  
    // Position in source document (for inline @mentions and #tags)
    source_anchor?: {
      document_id: UUID;
      block_id: UUID;            // Which text block contains the mention/tag
      offset_start: number;      // Character offset in ProseMirror content
      offset_end: number;
    };
  }
  
  enum LoomEdgeType {
    MENTION = 'mention',           // @mention \\u00E2\\u20AC\\u201D \"this block references that block\"
    TAG = 'tag',                   // #tag \\u00E2\\u20AC\\u201D \"this block is categorized as that tag\"
    SUB_TAG = 'sub_tag',           // Tag hierarchy \\u00E2\\u20AC\\u201D \"this tag is a sub-tag of that tag\"
    PARENT = 'parent',             // Structural \\u00E2\\u20AC\\u201D \"this block is a child of that block\"
    AI_SUGGESTED = 'ai_suggested', // AI-proposed link (DerivedContent until user confirms)
  }
  ```
  
  **Mention semantics (@)**
  
  - **[LM-LINK-001]** @mentions create `MENTION` edges in the Knowledge Graph. These are **RawContent** (user-authored, intentional).
  - **[LM-LINK-002]** @mentions are embedded in the rich-text editor flow (inline in ProseMirror/Tiptap content). The editor MUST render them as interactive links that navigate to the target block.
  - **[LM-LINK-003]** @mentions MUST be stable across renames. They reference target blocks by UUID, not by title text. If the target block is renamed, the mention display updates automatically.
  - **[LM-LINK-004]** Creating an @mention to a non-existent block MUST auto-create a new LoomBlock (`content_type: NOTE`) with that title.
  
  **Tag semantics (#)**
  
  - **[LM-TAG-001]** #tags create `TAG` edges in the Knowledge Graph. Tags are **RawContent** (user-authored categorization).
  - **[LM-TAG-002]** Tags are themselves LoomBlocks (`content_type: TAG_HUB`). A tag can carry its own rich-text content, sub-tags, and backlinks.
  - **[LM-TAG-003]** Tags MUST support hierarchical relationships via `SUB_TAG` edges. Example: `#project/alpha` creates a `SUB_TAG` edge from `#alpha` to `#project`.
  - **[LM-TAG-004]** Tags referenced inline in the editor MUST be rendered as interactive labels. Clicking a tag navigates to the tag\\u00E2\\u20AC\\u2122s LoomBlock, which shows its content and all tagged blocks as backlinks.
  - **[LM-TAG-005]** AI-suggested tags (from auto-tagging jobs) MUST be stored as `AI_SUGGESTED` edges (DerivedContent) until the user explicitly confirms them, at which point they become `TAG` edges (RawContent).
  
  **Backlink display**
  
  - **[LM-BACK-001]** Every LoomBlock surface MUST include a backlinks section showing all blocks that reference it via `MENTION` or `TAG` edges.
  - **[LM-BACK-002]** Backlinks are DerivedContent (computed from graph traversal). They MUST update reactively when new edges are created or deleted.
  - **[LM-BACK-003]** Backlinks SHOULD display a context snippet \\u00E2\\u20AC\\u201D the surrounding text from the source block where the mention/tag appears \\u00E2\\u20AC\\u201D so users can understand the relationship without navigating away.
  
  ---
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.134.md 10.12 Loom Integration Spec 6 Media & File Management: Cache-Tiered Asset Browsing
- CONTEXT_START_LINE: 59102
- CONTEXT_END_LINE: 59137
- CONTEXT_TOKEN: #### 6. Media & File Management: Cache-Tiered Asset Browsing
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 6. Media & File Management: Cache-Tiered Asset Browsing
  
  ##### 6.1 Import Pipeline
  
  **[LM-MEDIA-001]** Importing a file into a loom MUST:
  1. Compute `content_hash` (SHA-256) of the file.
  2. Check for duplicates within the workspace. If a match exists, navigate to the existing LoomBlock instead of creating a new one.
  3. Store the file as an Asset (\\u00C2\\u00A72.2.3.1).
  4. Create a LoomBlock linking to the Asset.
  5. Queue thumbnail/preview generation as a background job.
  6. Index the LoomBlock in the Shadow Workspace (\\u00C2\\u00A72.3.8).
  7. Emit a Flight Recorder event.
  
  **[LM-MEDIA-002]** Deduplication MUST be workspace-scoped. The same file MAY exist in multiple workspaces (different heaps) without dedup.
  
  ##### 6.2 Cache Tiers
  
  Handshake already defines Asset proxy/preview types (\\u00C2\\u00A72.2.3.1 ProxySettings, \\u00C2\\u00A710.10.7.3). The Heaper integration formalizes a three-tier cache model:
  
  | Tier | What | Where | When Available |
  |------|------|-------|----------------|
  | **Tier 0: Metadata** | LoomBlock record + title + tags + links + timestamps | Always local (SQLite/CRDT) | Always, even offline |
  | **Tier 1: Preview** | Thumbnail (image), waveform (audio), poster frame (video), first-page render (PDF) | Local cache, preferably SSD | After initial sync or background generation |
  | **Tier 2: Proxy** | Mid-resolution version suitable for browsing/editing (e.g., 2048px longest edge for images) | Local cache or on-demand download | On first open or prefetch |
  | **Tier 3: Original** | Full-resolution original file | Device that imported it; others fetch on demand | On explicit request or device with storage headroom |
  
  **[LM-CACHE-001]** Tier 0 (metadata) and Tier 1 (preview) MUST be replicated to all synced devices by default. This ensures offline browsing of the entire library on any device.
  
  **[LM-CACHE-002]** Tier 2 (proxy) and Tier 3 (original) replication is configurable per device. Devices with limited storage (phones) MAY be set to Tier 1 only.
  
  **[LM-CACHE-003]** Thumbnail generation MUST run as a mechanical job (\\u00C2\\u00A76.0 Mechanical Tool Bus principles) with Flight Recorder logging. It MUST NOT block the import flow.
  
  **[LM-CACHE-004]** Video files MUST support streaming from the sync server or source device without requiring full local download. This matches Heaper's \"video files can now be streamed if not in cache\" behavior.
  
  **[LM-CACHE-005]** The cache tier configuration MUST be persisted per-device in workspace settings. The storage trait (\\u00C2\\u00A72.3.13) MUST support querying replication state (\"which devices have Tier 3 for this asset?\").
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.134.md 10.12 Loom Integration Spec 7 Loom Views: Browsing Projections
- CONTEXT_START_LINE: 59173
- CONTEXT_END_LINE: 59202
- CONTEXT_TOKEN: #### 7. Loom Views: Browsing Projections
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 7. Loom Views: Browsing Projections
  
  ##### 7.1 View Types
  
  Heaper defines four core browsing views. These are DisplayContent projections (\\u00C2\\u00A72.2.2.3) over the same underlying LoomBlock data.
  
  | View | Query | Purpose | UX |
  |------|-------|---------|----|
  | **All** | All LoomBlocks, sorted by `updated_at` DESC | Chronological feed (like photo stream) | Infinite scroll, grid or list |
  | **Unlinked** | LoomBlocks with zero MENTION + TAG edges (backlink_count + mention_count + tag_count == 0) | Triage queue: items not yet organized | Items disappear when linked (satisfying \"sorting feels like progress\") |
  | **Sorted** | LoomBlocks grouped by their TAG and MENTION targets | Browse by structure: each group shows blocks under that tag/mention | Expandable group headers; each group is a mini-feed |
  | **Pins** | LoomBlocks where `pinned == true` | Quick access | Grid, user-reorderable |
  
  **[LM-VIEW-001]** All four views MUST be available as workspace-level surfaces. They are NOT separate stores \\u00E2\\u20AC\\u201D they query the same LoomBlock table with different filters/groupings.
  
  **[LM-VIEW-002]** The Unlinked view is the most important organizational tool. It MUST update in real-time as the user creates links. A block that gains its first MENTION or TAG edge MUST disappear from the Unlinked view immediately.
  
  **[LM-VIEW-003]** Views MUST support filtering by: content_type, file MIME type, date range, specific tags, specific mentions.
  
  **[LM-VIEW-004]** Views MUST support switching between grid layout (optimized for media browsing) and list layout (optimized for notes/documents).
  
  ##### 7.2 Integration with Existing Handshake Surfaces
  
  Loom views are a new surface family alongside Handshake's existing document editor, canvas, table, and chart surfaces. They follow the same \\u00C2\\u00A77.1.0 Cross-View Tool Integration rules:
  
  - Loom views MUST NOT introduce their own persistent storage or IDs.
  - LoomBlocks that are also Documents participate in the document editor.
  - LoomBlocks that contain Assets participate in the photo/media pipeline (\\u00C2\\u00A710.10).
  - Dragging a LoomBlock onto a Canvas creates a Canvas Node referencing the same entity (no copy).
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.134.md 10.12 Loom Integration Spec 9.3 Three-Tier Search Architecture
- CONTEXT_START_LINE: 59280
- CONTEXT_END_LINE: 59304
- CONTEXT_TOKEN: ##### 9.3 Three-Tier Search Architecture
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 9.3 Three-Tier Search Architecture
  
  | Tier | Engine | Available On | Capability |
  |------|--------|-------------|------------|
  | Tier 1 (local, instant) | SQLite FTS5 | SQLite + PostgreSQL | Offline keyword search; matches Heaper's current capability |
  | Tier 2 (server, relational) | PostgreSQL `tsvector` + GIN | PostgreSQL only | Ranked results, language-aware stemming, filter by graph relationships in same query |
  | Tier 3 (semantic) | Shadow Workspace embeddings (\\u00C2\\u00A72.3.8) | SQLite + PostgreSQL | Vector similarity search; feeds AI/RAG pipeline |
  
  ```sql
  -- Portable: full-text index for LoomBlocks
  -- SQLite: uses FTS5 virtual table
  -- PostgreSQL: uses tsvector column + GIN index
  
  -- PostgreSQL-enhanced query example:
  -- \"Find blocks tagged #project-alpha linked to blocks mentioning @Sarah, 
  --  created in last 30 days, sorted by backlink count\"
  --
  -- This is a single PostgreSQL query with recursive CTE.
  -- On SQLite, equivalent requires client-side graph traversal.
  ```
  
  **[LM-SEARCH-001]** The search API MUST be backend-agnostic. The storage trait exposes `search_loom_blocks(query, filters) -> Vec<LoomBlockSearchResult>`. The implementation varies by backend.
  
  **[LM-SEARCH-002]** On PostgreSQL, search results MUST be filterable by graph relationships (tags, mentions, backlink depth) within the query. This is a key improvement over Heaper's client-side-only search.
  ```

#### ANCHOR 6
- SPEC_ANCHOR: Handshake_Master_Spec_v02.134.md 11.5.12 FR-EVT-LOOM-001..012 (Loom Surface Events) (Normative) [ADD v02.130]
- CONTEXT_START_LINE: 64024
- CONTEXT_END_LINE: 64138
- CONTEXT_TOKEN: ### 11.5.12 FR-EVT-LOOM-001..012 (Loom Surface Events) (Normative) [ADD v02.130]
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 11.5.12 FR-EVT-LOOM-001..012 (Loom Surface Events) (Normative) [ADD v02.130]
  
  Events for LoomBlock lifecycle, edge creation/deletion, deduplication, preview generation, AI tag suggestion workflows, and Loom view/search interactions (\\u00C2\\u00A710.12).
  
  ```ts
  // FR-EVT-LOOM-001: LoomBlock created
  interface LoomBlockCreatedEvent extends FlightRecorderEventBase {
    type: 'loom_block_created';
    block_id: string;
    workspace_id: string;
    content_type: string;
    asset_id?: string | null;
    content_hash?: string | null;
  }
  
  // FR-EVT-LOOM-002: LoomBlock updated (metadata change)
  interface LoomBlockUpdatedEvent extends FlightRecorderEventBase {
    type: 'loom_block_updated';
    block_id: string;
    fields_changed: string[];
    updated_by: 'user' | 'ai';
  }
  
  // FR-EVT-LOOM-003: LoomBlock deleted
  interface LoomBlockDeletedEvent extends FlightRecorderEventBase {
    type: 'loom_block_deleted';
    block_id: string;
    workspace_id: string;
    content_type: string;
    had_asset: boolean;
  }
  
  // FR-EVT-LOOM-004: Loom edge created (mention/tag/link)
  interface LoomEdgeCreatedEvent extends FlightRecorderEventBase {
    type: 'loom_edge_created';
    edge_id: string;
    source_block_id: string;
    target_block_id: string;
    edge_type: 'mention' | 'tag' | 'sub_tag' | 'parent' | 'ai_suggested';
    created_by: 'user' | 'ai';
  }
  
  // FR-EVT-LOOM-005: Loom edge deleted
  interface LoomEdgeDeletedEvent extends FlightRecorderEventBase {
    type: 'loom_edge_deleted';
    edge_id: string;
    edge_type: 'mention' | 'tag' | 'sub_tag' | 'parent' | 'ai_suggested';
    deleted_by: 'user' | 'ai';
  }
  
  // FR-EVT-LOOM-006: Dedup hit on import
  interface LoomDedupHitEvent extends FlightRecorderEventBase {
    type: 'loom_dedup_hit';
    workspace_id: string;
    content_hash: string;
    existing_block_id: string;
    attempted_filename: string;
  }
  
  // FR-EVT-LOOM-007: Preview generated (thumbnail/proxy)
  interface LoomPreviewGeneratedEvent extends FlightRecorderEventBase {
    type: 'loom_preview_generated';
    block_id: string;
    asset_id: string;
    preview_tier: 0 | 1 | 2;        // 0=original, 1=thumbnail, 2=proxy
    format: string;                // e.g. 'jpg', 'webp', 'mp4'
    duration_ms: number;
  }
  
  // FR-EVT-LOOM-008: AI tag suggested (auto-tag job completed)
  interface LoomAiTagSuggestedEvent extends FlightRecorderEventBase {
    type: 'loom_ai_tag_suggested';
    block_id: string;
    job_id: string;
    suggested_tags: string[];
    model_id: string;
  }
  
  // FR-EVT-LOOM-009: AI tag accepted
  interface LoomAiTagAcceptedEvent extends FlightRecorderEventBase {
    type: 'loom_ai_tag_accepted';
    block_id: string;
    edge_id: string;
    tag_name: string;
    was_ai_suggested: boolean;
  }
  
  // FR-EVT-LOOM-010: AI tag rejected
  interface LoomAiTagRejectedEvent extends FlightRecorderEventBase {
    type: 'loom_ai_tag_rejected';
    block_id: string;
    tag_name: string;
    was_ai_suggested: boolean;
  }
  
  // FR-EVT-LOOM-011: Loom view queried / opened
  interface LoomViewQueriedEvent extends FlightRecorderEventBase {
    type: 'loom_view_queried';
    workspace_id: string;
    view_type: 'all' | 'unlinked' | 'sorted' | 'pins';
    filter_count: number;
    result_count: number;
    duration_ms: number;
  }
  
  // FR-EVT-LOOM-012: Loom search executed
  interface LoomSearchExecutedEvent extends FlightRecorderEventBase {
    type: 'loom_search_executed';
    workspace_id: string;
    query_length: number;
    tier_used: 1 | 2 | 3;          // 1=FTS, 2=PG full-text, 3=hybrid semantic
    result_count: number;
    duration_ms: number;
  }
  ```
  ```
