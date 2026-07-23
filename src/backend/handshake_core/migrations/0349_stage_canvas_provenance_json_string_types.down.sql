-- Restore the 0348 constraint shape. The down migration intentionally
-- preserves the Stage provenance columns and their data.

ALTER TABLE loom_canvas_placements
    DROP CONSTRAINT IF EXISTS loom_canvas_placements_stage_provenance_key_check,
    ADD CONSTRAINT loom_canvas_placements_stage_provenance_key_check
        CHECK (
            (stage_provenance_key IS NULL AND stage_provenance IS NULL)
            OR (
                stage_provenance_key IS NOT NULL
                AND stage_provenance IS NOT NULL
                AND stage_provenance_key ~ '^[0-9a-f]{64}$'
                AND jsonb_typeof(stage_provenance) = 'object'
                AND stage_provenance ?& ARRAY[
                    'schema_id', 'artifact_id', 'sha256', 'manifest_ref',
                    'causal_action_id'
                ]
                AND stage_provenance - ARRAY[
                    'schema_id', 'artifact_id', 'sha256', 'manifest_ref',
                    'causal_action_id'
                ] = '{}'::jsonb
                AND stage_provenance ->> 'schema_id' = 'handshake.canvas-stage-capture-ref.v1'
                AND stage_provenance ->> 'artifact_id' IS NOT NULL
                AND stage_provenance ->> 'artifact_id' = btrim(stage_provenance ->> 'artifact_id')
                AND stage_provenance ->> 'artifact_id' <> ''
                AND stage_provenance ->> 'sha256' ~ '^[0-9a-f]{64}$'
                AND stage_provenance ->> 'manifest_ref' IS NOT NULL
                AND stage_provenance ->> 'manifest_ref' = btrim(stage_provenance ->> 'manifest_ref')
                AND stage_provenance ->> 'manifest_ref' <> ''
                AND stage_provenance ->> 'causal_action_id' IS NOT NULL
                AND stage_provenance ->> 'causal_action_id' = btrim(stage_provenance ->> 'causal_action_id')
                AND stage_provenance ->> 'causal_action_id' <> ''
            )
        );
