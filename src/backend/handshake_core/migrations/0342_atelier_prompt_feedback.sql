-- MT-020: deterministic prompt-feedback kernel (first slice).
-- PostgreSQL/EventLedger is the authority. Prompt cases, review verdicts,
-- versioned rule packs, deterministic rewrites (plan + trace), and materialized
-- JSONL export receipts. JSONL export bytes live in the ArtifactStore behind a
-- portable artifact:// ref; this table stores only the ref + content hash.
-- Reuses atelier_is_native_portable_ref() (migration 0340) for ref columns.

CREATE TABLE IF NOT EXISTS atelier_prompt_feedback_case (
    case_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id TEXT NOT NULL,
    source_system TEXT NOT NULL,
    adapter_id TEXT NOT NULL,
    source_iteration_id TEXT,
    source_case_id TEXT NOT NULL,
    source_recipe_id TEXT,
    segment TEXT NOT NULL,
    cell TEXT NOT NULL,
    framing TEXT NOT NULL,
    clothing_state TEXT NOT NULL,
    render_stack TEXT NOT NULL,
    identity_judgement_allowed BOOLEAN NOT NULL,
    prompt_quality_review_allowed BOOLEAN NOT NULL,
    positive_prompt TEXT NOT NULL,
    negative_prompt TEXT NOT NULL,
    micro_gate TEXT,
    expected_failure TEXT,
    image_artifact_ref TEXT,
    sheet_artifact_ref TEXT,
    axes JSONB NOT NULL DEFAULT '{}'::jsonb,
    hardcore_fields JSONB NOT NULL DEFAULT '{}'::jsonb,
    imported_by TEXT NOT NULL,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_prompt_feedback_case_project CHECK (
        btrim(project_id) = project_id AND project_id <> ''
    ),
    CONSTRAINT chk_atelier_prompt_feedback_case_source_system CHECK (
        btrim(source_system) = source_system AND source_system <> ''
    ),
    CONSTRAINT chk_atelier_prompt_feedback_case_adapter CHECK (
        btrim(adapter_id) = adapter_id AND adapter_id <> ''
    ),
    CONSTRAINT chk_atelier_prompt_feedback_case_source_case CHECK (
        btrim(source_case_id) = source_case_id AND source_case_id <> ''
    ),
    CONSTRAINT chk_atelier_prompt_feedback_case_segment CHECK (
        btrim(segment) = segment AND segment <> ''
    ),
    CONSTRAINT chk_atelier_prompt_feedback_case_cell CHECK (
        btrim(cell) = cell AND cell <> ''
    ),
    CONSTRAINT chk_atelier_prompt_feedback_case_framing CHECK (
        btrim(framing) = framing AND framing <> ''
    ),
    CONSTRAINT chk_atelier_prompt_feedback_case_clothing CHECK (
        btrim(clothing_state) = clothing_state AND clothing_state <> ''
    ),
    CONSTRAINT chk_atelier_prompt_feedback_case_render_stack CHECK (
        btrim(render_stack) = render_stack AND render_stack <> ''
    ),
    CONSTRAINT chk_atelier_prompt_feedback_case_imported_by CHECK (
        btrim(imported_by) = imported_by AND imported_by <> ''
    ),
    CONSTRAINT chk_atelier_prompt_feedback_case_image_ref CHECK (
        image_artifact_ref IS NULL OR atelier_is_native_portable_ref(image_artifact_ref)
    ),
    CONSTRAINT chk_atelier_prompt_feedback_case_sheet_ref CHECK (
        sheet_artifact_ref IS NULL OR atelier_is_native_portable_ref(sheet_artifact_ref)
    ),
    CONSTRAINT chk_atelier_prompt_feedback_case_axes CHECK (
        jsonb_typeof(axes) = 'object'
    ),
    CONSTRAINT chk_atelier_prompt_feedback_case_hardcore CHECK (
        jsonb_typeof(hardcore_fields) = 'object'
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_atelier_prompt_feedback_case_source
    ON atelier_prompt_feedback_case(adapter_id, source_case_id);

CREATE INDEX IF NOT EXISTS idx_atelier_prompt_feedback_case_grouping
    ON atelier_prompt_feedback_case(segment, cell, render_stack, created_at_utc DESC);

CREATE INDEX IF NOT EXISTS idx_atelier_prompt_feedback_case_project
    ON atelier_prompt_feedback_case(project_id, created_at_utc DESC);

CREATE TABLE IF NOT EXISTS atelier_prompt_feedback_verdict (
    verdict_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    case_id UUID NOT NULL REFERENCES atelier_prompt_feedback_case(case_id) ON DELETE CASCADE,
    reviewer_kind TEXT NOT NULL CHECK (
        reviewer_kind IN ('operator', 'model', 'subagent', 'validator', 'script')
    ),
    reviewer_id TEXT NOT NULL,
    verdict_kind TEXT NOT NULL CHECK (
        verdict_kind IN ('success', 'watch', 'failure', 'reject', 'diagnostic')
    ),
    failure_class TEXT,
    failure_tags JSONB NOT NULL DEFAULT '[]'::jsonb,
    is_identity_judgement BOOLEAN NOT NULL DEFAULT FALSE,
    note TEXT,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_prompt_feedback_verdict_reviewer_id CHECK (
        btrim(reviewer_id) = reviewer_id AND reviewer_id <> ''
    ),
    CONSTRAINT chk_atelier_prompt_feedback_verdict_failure_class CHECK (
        failure_class IS NULL OR (btrim(failure_class) = failure_class AND failure_class <> '')
    ),
    CONSTRAINT chk_atelier_prompt_feedback_verdict_tags CHECK (
        jsonb_typeof(failure_tags) = 'array'
    ),
    CONSTRAINT chk_atelier_prompt_feedback_verdict_note CHECK (
        note IS NULL OR (btrim(note) = note AND note <> '')
    )
);

CREATE INDEX IF NOT EXISTS idx_atelier_prompt_feedback_verdict_case
    ON atelier_prompt_feedback_verdict(case_id, created_at_utc DESC);

CREATE TABLE IF NOT EXISTS atelier_prompt_feedback_rule_pack (
    rule_pack_id TEXT NOT NULL,
    version INTEGER NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    rules JSONB NOT NULL DEFAULT '[]'::jsonb,
    content_hash TEXT NOT NULL,
    registered_by TEXT NOT NULL,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (rule_pack_id, version),
    CONSTRAINT chk_atelier_prompt_feedback_rule_pack_id CHECK (
        btrim(rule_pack_id) = rule_pack_id AND rule_pack_id <> ''
    ),
    CONSTRAINT chk_atelier_prompt_feedback_rule_pack_version CHECK (version >= 1),
    CONSTRAINT chk_atelier_prompt_feedback_rule_pack_rules CHECK (
        jsonb_typeof(rules) = 'array'
    ),
    CONSTRAINT chk_atelier_prompt_feedback_rule_pack_registered_by CHECK (
        btrim(registered_by) = registered_by AND registered_by <> ''
    )
);

CREATE TABLE IF NOT EXISTS atelier_prompt_feedback_rewrite (
    rewrite_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    case_id UUID NOT NULL REFERENCES atelier_prompt_feedback_case(case_id) ON DELETE CASCADE,
    source_case_id TEXT NOT NULL,
    rule_pack_id TEXT NOT NULL,
    rule_pack_version INTEGER NOT NULL,
    input_hash TEXT NOT NULL,
    output_hash TEXT NOT NULL,
    changed_fields JSONB NOT NULL DEFAULT '[]'::jsonb,
    rewritten_positive_prompt TEXT NOT NULL,
    rewritten_negative_prompt TEXT NOT NULL,
    outcome JSONB NOT NULL,
    planned_by TEXT NOT NULL,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT fk_atelier_prompt_feedback_rewrite_rule_pack
        FOREIGN KEY (rule_pack_id, rule_pack_version)
        REFERENCES atelier_prompt_feedback_rule_pack(rule_pack_id, version)
        ON DELETE RESTRICT,
    CONSTRAINT chk_atelier_prompt_feedback_rewrite_input_hash CHECK (
        btrim(input_hash) = input_hash AND input_hash <> ''
    ),
    CONSTRAINT chk_atelier_prompt_feedback_rewrite_output_hash CHECK (
        btrim(output_hash) = output_hash AND output_hash <> ''
    ),
    CONSTRAINT chk_atelier_prompt_feedback_rewrite_changed CHECK (
        jsonb_typeof(changed_fields) = 'array'
    ),
    CONSTRAINT chk_atelier_prompt_feedback_rewrite_outcome CHECK (
        jsonb_typeof(outcome) = 'object'
    ),
    CONSTRAINT chk_atelier_prompt_feedback_rewrite_planned_by CHECK (
        btrim(planned_by) = planned_by AND planned_by <> ''
    )
);

-- Deterministic idempotency: the same case + rule-pack version + input hash can
-- only produce one persisted rewrite (byte-stable output).
CREATE UNIQUE INDEX IF NOT EXISTS ux_atelier_prompt_feedback_rewrite_determinism
    ON atelier_prompt_feedback_rewrite(case_id, rule_pack_id, rule_pack_version, input_hash);

CREATE TABLE IF NOT EXISTS atelier_prompt_feedback_export (
    export_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    rule_pack_id TEXT NOT NULL,
    rule_pack_version INTEGER NOT NULL,
    artifact_ref TEXT NOT NULL,
    manifest_ref TEXT,
    content_hash TEXT NOT NULL,
    byte_len BIGINT NOT NULL,
    row_count INTEGER NOT NULL,
    source_case_ids JSONB NOT NULL DEFAULT '[]'::jsonb,
    rewrite_ids JSONB NOT NULL DEFAULT '[]'::jsonb,
    exported_by TEXT NOT NULL,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_prompt_feedback_export_artifact_ref CHECK (
        atelier_is_native_portable_ref(artifact_ref)
    ),
    CONSTRAINT chk_atelier_prompt_feedback_export_manifest_ref CHECK (
        manifest_ref IS NULL OR atelier_is_native_portable_ref(manifest_ref)
    ),
    CONSTRAINT chk_atelier_prompt_feedback_export_content_hash CHECK (
        btrim(content_hash) = content_hash AND content_hash <> ''
    ),
    CONSTRAINT chk_atelier_prompt_feedback_export_byte_len CHECK (byte_len >= 0),
    CONSTRAINT chk_atelier_prompt_feedback_export_row_count CHECK (row_count >= 0),
    CONSTRAINT chk_atelier_prompt_feedback_export_source_case_ids CHECK (
        jsonb_typeof(source_case_ids) = 'array'
    ),
    CONSTRAINT chk_atelier_prompt_feedback_export_rewrite_ids CHECK (
        jsonb_typeof(rewrite_ids) = 'array'
    ),
    CONSTRAINT chk_atelier_prompt_feedback_export_exported_by CHECK (
        btrim(exported_by) = exported_by AND exported_by <> ''
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_atelier_prompt_feedback_export_content
    ON atelier_prompt_feedback_export(rule_pack_id, rule_pack_version, content_hash);

CREATE INDEX IF NOT EXISTS idx_atelier_prompt_feedback_export_recent
    ON atelier_prompt_feedback_export(created_at_utc DESC);
