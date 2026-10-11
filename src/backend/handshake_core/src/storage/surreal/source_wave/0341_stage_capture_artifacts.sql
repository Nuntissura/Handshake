-- WP-KERNEL-012 MT-066: the Stage capture artifact store.
--
-- The Stage pane (Pillar 17) captures an inline text artifact — a document, a
-- selection, a canvas node, or an atelier item — as an evidence-grade PROVENANCE
-- descriptor (metadata, NOT content bytes). The frontend embed-back leg
-- (`stage_interop::StageArtifactRef` / `StageManifest::is_evidence_grade`)
-- REFUSES an artifact whose sha256 OR manifest_ref is empty
-- (`StageInteropError::ProvenanceMissing`), so both are stored NON-EMPTY and
-- CHECK-enforced here: `content_sha256` is a lowercase 64-hex digest and
-- `manifest_ref` is a non-blank `manifest://{artifact_id}` reference.
--
-- Scope (MT-066, minimal viable): INLINE TEXT captures only. The captured value
-- lives in `content_json` (JSONB authority) with its canonical-JSON
-- `content_sha256`, mirroring `knowledge_rich_documents` (0140). Binary/blob
-- captures (e.g. `image/png` via an ArtifactStore handle) are DEFERRED — the
-- frontend GET is metadata-only so binary defers cleanly.
--
-- NOT a `knowledge_` table: Stage capture is Pillar 17, distinct from the
-- ProjectKnowledgeIndex (WP-009) namespace, so it is intentionally NOT
-- registered in `knowledge_schema_registry` (whose CHECK requires a
-- `knowledge_` table-name prefix — a stage table would fail that constraint).
-- It is a standalone additive table, mirroring the sibling
-- `calendar_activity_spans` (0340). PostgreSQL authority only, no SQLite.

CREATE TABLE IF NOT EXISTS stage_capture_artifacts (
    artifact_id TEXT PRIMARY KEY
        CHECK (artifact_id ~ '^STGA-[0-9a-f]{32}$'),
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    content_kind TEXT NOT NULL
        CHECK (content_kind IN ('document', 'selection', 'canvas_node', 'atelier_item')),
    label TEXT NOT NULL DEFAULT '',
    content_type TEXT NOT NULL,
    -- The captured value (inline text authority; ProseMirror node, selection
    -- payload, node reference, or atelier item shape).
    content_json JSONB NOT NULL,
    -- Canonical-JSON SHA-256 of content_json (same canonical form as kernel
    -- ContextBundle / knowledge_rich_documents hashing, so it is replayable).
    content_sha256 TEXT NOT NULL
        CHECK (content_sha256 ~ '^[0-9a-f]{64}$'),
    -- The provenance manifest descriptor:
    -- {schema, sha256, manifest_ref, content_type, source_ref}.
    manifest JSONB NOT NULL DEFAULT '{}'::jsonb,
    -- `manifest://{artifact_id}` — the manifest record reference. The
    -- evidence-grade twin of the frontend gate: never blank.
    manifest_ref TEXT NOT NULL
        CHECK (btrim(manifest_ref) <> ''),
    -- Optional back-reference to the origin (note/canvas/atelier ref).
    source_ref TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_stage_capture_artifacts_workspace
    ON stage_capture_artifacts (workspace_id);

CREATE INDEX IF NOT EXISTS idx_stage_capture_artifacts_content_sha256
    ON stage_capture_artifacts (content_sha256);
