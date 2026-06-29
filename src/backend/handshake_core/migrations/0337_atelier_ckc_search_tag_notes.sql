-- WP-CKC-posekit-overhaul MT-011 CKC search and rich tag notes.
-- Tag notes are first-class CKC data, separate from character sheet notes,
-- media review notes, and collection notes. Search uses pg_trgm for fuzzy
-- matching and the Handshake public pgvector invariant for native semantic
-- CKC projections when an embedding model is configured.

DO $$
DECLARE
    existing_schema TEXT;
BEGIN
    SELECT n.nspname
      INTO existing_schema
      FROM pg_extension e
      JOIN pg_namespace n ON n.oid = e.extnamespace
     WHERE e.extname = 'pg_trgm';

    IF existing_schema IS NULL THEN
        CREATE EXTENSION pg_trgm WITH SCHEMA public;
    END IF;
END $$;

DO $$
DECLARE
    existing_schema TEXT;
BEGIN
    SELECT n.nspname
      INTO existing_schema
      FROM pg_extension e
      JOIN pg_namespace n ON n.oid = e.extnamespace
     WHERE e.extname = 'vector';

    IF existing_schema IS NULL THEN
        CREATE EXTENSION vector WITH SCHEMA public;
    ELSIF existing_schema <> 'public' THEN
        RAISE EXCEPTION
            'Handshake requires vector extension in schema public; found schema %',
            existing_schema
            USING ERRCODE = 'invalid_schema_name';
    END IF;
END $$;

CREATE OR REPLACE FUNCTION public.atelier_trgm_similarity(left_text TEXT, right_text TEXT)
RETURNS REAL
LANGUAGE plpgsql
STABLE
PARALLEL SAFE
AS $$
DECLARE
    extension_schema TEXT;
    score REAL;
BEGIN
    SELECT n.nspname
      INTO extension_schema
      FROM pg_extension e
      JOIN pg_namespace n ON n.oid = e.extnamespace
     WHERE e.extname = 'pg_trgm';

    IF extension_schema IS NULL THEN
        RETURN 0.0;
    END IF;

    EXECUTE format('SELECT %I.similarity($1, $2)', extension_schema)
       INTO score
      USING left_text, right_text;
    RETURN COALESCE(score, 0.0);
END $$;

CREATE TABLE IF NOT EXISTS atelier_tag_note (
    tag_note_id     UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tag_id          UUID NOT NULL REFERENCES atelier_tag(tag_id) ON DELETE CASCADE,
    scope_ref       TEXT,
    note            TEXT NOT NULL,
    updated_by      TEXT NOT NULL,
    created_at_utc  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_tag_note_scope_ref_trimmed
        CHECK (scope_ref IS NULL OR (btrim(scope_ref) = scope_ref AND scope_ref <> '')),
    CONSTRAINT chk_atelier_tag_note_note_trimmed
        CHECK (btrim(note) = note AND note <> ''),
    CONSTRAINT chk_atelier_tag_note_updated_by_trimmed
        CHECK (btrim(updated_by) = updated_by AND updated_by <> '')
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_atelier_tag_note_tag_scope
    ON atelier_tag_note(tag_id, COALESCE(scope_ref, ''));

CREATE INDEX IF NOT EXISTS idx_atelier_tag_note_scope_ref
    ON atelier_tag_note(scope_ref);

CREATE TABLE IF NOT EXISTS atelier_ckc_search_projection (
    target_ref       TEXT PRIMARY KEY,
    target_kind      TEXT NOT NULL,
    search_text_hash TEXT NOT NULL,
    embedding        public.vector(768),
    embedding_model  TEXT,
    indexed_at_utc   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_atelier_ckc_search_projection_embedding
    ON atelier_ckc_search_projection USING hnsw (embedding public.vector_cosine_ops);

CREATE INDEX IF NOT EXISTS idx_atelier_ckc_search_projection_kind
    ON atelier_ckc_search_projection(target_kind);
