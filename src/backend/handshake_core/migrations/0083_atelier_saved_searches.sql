-- WP-KERNEL-005 MT-047 saved searches and retrieval projections.
-- Durable filters live in PostgreSQL; execution is a read-only projection over
-- existing media, tag, review metadata, collection, and palette tables.

CREATE TABLE IF NOT EXISTS atelier_saved_search (
    saved_search_id   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name              TEXT NOT NULL UNIQUE,
    include_tags_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    exclude_tags_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    min_rating        SMALLINT CHECK (min_rating IS NULL OR min_rating BETWEEN 0 AND 5),
    favorite          BOOLEAN,
    color_hex         TEXT,
    scope_kind        TEXT NOT NULL DEFAULT 'all_media',
    scope_id          UUID,
    view_mode         TEXT NOT NULL DEFAULT 'NSFW',
    created_by        TEXT NOT NULL,
    created_at_utc    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_saved_search_name_trimmed
        CHECK (btrim(name) = name AND name <> ''),
    CONSTRAINT chk_atelier_saved_search_created_by_trimmed
        CHECK (btrim(created_by) = created_by AND created_by <> ''),
    CONSTRAINT chk_atelier_saved_search_include_tags_array
        CHECK (jsonb_typeof(include_tags_json) = 'array'),
    CONSTRAINT chk_atelier_saved_search_exclude_tags_array
        CHECK (jsonb_typeof(exclude_tags_json) = 'array'),
    CONSTRAINT chk_atelier_saved_search_color_hex
        CHECK (color_hex IS NULL OR color_hex ~ '^#[0-9a-f]{6}$'),
    CONSTRAINT chk_atelier_saved_search_scope
        CHECK (
            (scope_kind = 'all_media' AND scope_id IS NULL)
            OR (scope_kind = 'collection' AND scope_id IS NOT NULL)
        ),
    CONSTRAINT chk_atelier_saved_search_view_mode
        CHECK (view_mode IN ('NSFW', 'SFW'))
);

CREATE INDEX IF NOT EXISTS idx_atelier_saved_search_updated
    ON atelier_saved_search(updated_at_utc DESC);

DROP VIEW IF EXISTS atelier_saved_search_retrieval_projection;

CREATE OR REPLACE VIEW atelier_saved_search_retrieval_projection AS
WITH media_tags AS (
    SELECT mat.asset_id,
           jsonb_agg(t.text ORDER BY t.text) AS tags_json
    FROM atelier_media_asset_tag mat
    JOIN atelier_tag t
      ON t.tag_id = mat.tag_id
    GROUP BY mat.asset_id
),
base_projection AS (
    SELECT ss.saved_search_id,
           ss.name AS saved_search_name,
           ma.asset_id,
           ma.content_hash,
           ma.artifact_ref,
           concat('atelier://image/', ma.asset_id::text) AS jump_target,
           COALESCE(mt.tags_json, '[]'::jsonb) AS tags_json,
           COALESCE(mrm.favorite, false) AS favorite,
           COALESCE(mrm.rating, 0::smallint)::smallint AS rating,
           ss.min_rating,
           ss.favorite AS required_favorite,
           ss.color_hex,
           ss.scope_kind,
           ss.scope_id,
           ss.view_mode,
           CASE
               WHEN lower(COALESCE(ma.source_provenance, '')) ~ 'content[_ -]?tier[^a-z0-9]+adult[_ -]?explicit' THEN 'adult_explicit'
               WHEN lower(COALESCE(ma.source_provenance, '')) ~ 'content[_ -]?tier[^a-z0-9]+adult[_ -]?soft' THEN 'adult_soft'
               WHEN lower(COALESCE(ma.source_provenance, '')) ~ 'content[_ -]?tier[^a-z0-9]+sfw' THEN 'sfw'
               ELSE NULL
           END AS content_tier,
           (
               SELECT lower(color.value->>'hex')
               FROM jsonb_array_elements(COALESCE(sp.palette_json->'dominant', '[]'::jsonb)) AS color(value)
               WHERE ss.color_hex IS NOT NULL
                 AND lower(color.value->>'hex') = ss.color_hex
               LIMIT 1
           ) AS matched_color_hex,
           ma.created_at_utc
    FROM atelier_saved_search ss
    JOIN atelier_media_asset ma
      ON TRUE
    LEFT JOIN media_tags mt
      ON mt.asset_id = ma.asset_id
    LEFT JOIN atelier_media_review_metadata mrm
      ON mrm.asset_id = ma.asset_id
    LEFT JOIN atelier_similarity_projection sp
      ON sp.asset_internal_id = ma.asset_id
)
SELECT saved_search_id,
       saved_search_name,
       asset_id,
       content_hash,
       artifact_ref,
       jump_target,
       tags_json,
       favorite,
       rating,
       color_hex,
       matched_color_hex,
       scope_kind,
       scope_id,
       view_mode,
       content_tier,
       created_at_utc
FROM base_projection bp
WHERE (bp.min_rating IS NULL OR bp.rating >= bp.min_rating)
  AND (bp.required_favorite IS NULL OR bp.favorite = bp.required_favorite)
  AND (
      bp.color_hex IS NULL
      OR bp.matched_color_hex = bp.color_hex
  )
  AND (
      bp.scope_kind = 'all_media'
      OR EXISTS (
          SELECT 1
          FROM atelier_collection_item ci
          WHERE ci.collection_id = bp.scope_id
            AND ci.asset_id = bp.asset_id
      )
  )
  AND NOT EXISTS (
      SELECT 1
      FROM jsonb_array_elements_text(
          (SELECT include_tags_json FROM atelier_saved_search ss WHERE ss.saved_search_id = bp.saved_search_id)
      ) AS required_tag(text)
      WHERE NOT EXISTS (
          SELECT 1
          FROM jsonb_array_elements_text(bp.tags_json) AS actual_tag(text)
          WHERE actual_tag.text = required_tag.text
      )
  )
  AND NOT EXISTS (
      SELECT 1
      FROM jsonb_array_elements_text(
          (SELECT exclude_tags_json FROM atelier_saved_search ss WHERE ss.saved_search_id = bp.saved_search_id)
      ) AS excluded_tag(text)
      WHERE EXISTS (
          SELECT 1
          FROM jsonb_array_elements_text(bp.tags_json) AS actual_tag(text)
          WHERE actual_tag.text = excluded_tag.text
      )
  )
  AND (bp.view_mode <> 'SFW' OR bp.content_tier = 'sfw');
