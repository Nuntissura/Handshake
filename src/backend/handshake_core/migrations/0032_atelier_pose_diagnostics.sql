-- WP-KERNEL-005 Pose-ComfyUI + Diagnostics domain tables.
-- PostgreSQL authority only (SQLite forbidden, MT-004); idempotent;
-- applied transactionally under an advisory lock by AtelierStore::ensure_schema.
-- Domains: pose/rig artifacts (MT-PoseKit), ComfyUI custom-node intake (S6.9 / MT-202),
-- sourcing-spec + handler matrix (S6.12 / MT-201), media transcript/caption (S6.11 / MT-203),
-- media-downloader-v2 (S6.10 / MT-204), command-corpus parity (S10.19 / MT-206),
-- stealth reference window (S10.18 / MT-205). FKs into 0030 foundation
-- (atelier_character.internal_id, atelier_media_asset.asset_id).

------------------------------------------------------------------------------
-- POSE / RIG (MT-PoseKit). FKs to atelier_character / atelier_media_asset.
------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS atelier_pose_rig (
    rig_id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    character_internal_id UUID NOT NULL REFERENCES atelier_character(internal_id) ON DELETE CASCADE,
    source_asset_id       UUID REFERENCES atelier_media_asset(asset_id) ON DELETE SET NULL,
    source_ref            TEXT NOT NULL,
    content_hash          TEXT NOT NULL,
    canvas_width          INTEGER NOT NULL,
    canvas_height         INTEGER NOT NULL,
    detector_provider     TEXT NOT NULL,
    detector_status       TEXT NOT NULL CHECK (detector_status IN ('detected','fallback','failed')),
    keypoints_json        JSONB NOT NULL,
    sidecar_ref           TEXT,
    created_at_utc        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (character_internal_id, source_ref, content_hash)
);
CREATE INDEX IF NOT EXISTS idx_atelier_pose_rig_character ON atelier_pose_rig(character_internal_id, created_at_utc DESC);
CREATE INDEX IF NOT EXISTS idx_atelier_pose_rig_source_asset ON atelier_pose_rig(source_asset_id);
CREATE INDEX IF NOT EXISTS idx_atelier_pose_rig_hash ON atelier_pose_rig(content_hash);

CREATE TABLE IF NOT EXISTS atelier_pose_head_pose (
    rig_id    UUID PRIMARY KEY REFERENCES atelier_pose_rig(rig_id) ON DELETE CASCADE,
    yaw_deg   DOUBLE PRECISION NOT NULL,
    pitch_deg DOUBLE PRECISION NOT NULL,
    roll_deg  DOUBLE PRECISION NOT NULL,
    quat_x    DOUBLE PRECISION NOT NULL,
    quat_y    DOUBLE PRECISION NOT NULL,
    quat_z    DOUBLE PRECISION NOT NULL,
    quat_w    DOUBLE PRECISION NOT NULL,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS atelier_pose_calibration (
    rig_id         UUID PRIMARY KEY REFERENCES atelier_pose_rig(rig_id) ON DELETE CASCADE,
    state          TEXT NOT NULL CHECK (state IN ('unresolved','resolved')),
    block_reason   TEXT,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS atelier_identity_profile (
    profile_id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    character_internal_id UUID NOT NULL REFERENCES atelier_character(internal_id) ON DELETE CASCADE,
    seq                   BIGINT NOT NULL,
    kind                  TEXT NOT NULL CHECK (kind IN ('face','reference')),
    reference_asset_id    UUID REFERENCES atelier_media_asset(asset_id) ON DELETE SET NULL,
    reference_ref         TEXT NOT NULL,
    provenance            TEXT NOT NULL DEFAULT '',
    created_at_utc        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (character_internal_id, seq)
);
CREATE INDEX IF NOT EXISTS idx_atelier_identity_profile_character ON atelier_identity_profile(character_internal_id, kind, seq DESC);

------------------------------------------------------------------------------
-- COMFYUI CUSTOM-NODE INTAKE (S6.9 / MT-202). Governed records only.
------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS atelier_comfy_bridge_probe (
    probe_id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_run_id         UUID NOT NULL UNIQUE,
    node_class_id           TEXT NOT NULL,
    detected                BOOLEAN NOT NULL DEFAULT FALSE,
    bridge_protocol_version TEXT,
    node_instance_ids       JSONB NOT NULL DEFAULT '[]'::jsonb,
    probe_outcome           TEXT NOT NULL CHECK (probe_outcome IN ('bridge_present','bridge_absent','bridge_incompatible')),
    fallback_reason         TEXT,
    probed_at_utc           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT atelier_comfy_probe_fallback_ck CHECK (
        probe_outcome = 'bridge_present' OR fallback_reason IS NOT NULL
    )
);
CREATE INDEX IF NOT EXISTS idx_atelier_comfy_probe_outcome ON atelier_comfy_bridge_probe(probe_outcome);

CREATE TABLE IF NOT EXISTS atelier_comfy_capability_registration (
    registration_id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_run_id         UUID NOT NULL UNIQUE,
    node_class_id           TEXT NOT NULL,
    bridge_protocol_version TEXT NOT NULL,
    capability_grant_ref    TEXT NOT NULL,
    consent_decision_ref    TEXT,
    registered_at_utc       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS atelier_comfy_declared_output (
    declared_output_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    registration_id    UUID NOT NULL REFERENCES atelier_comfy_capability_registration(registration_id) ON DELETE CASCADE,
    seq                BIGINT NOT NULL,
    output_slot        TEXT NOT NULL,
    media_kind         TEXT NOT NULL CHECK (media_kind IN ('image','mask','latent_preview','video','sidecar_json')),
    expected_mime      TEXT NOT NULL,
    routing_intent     TEXT NOT NULL CHECK (routing_intent IN ('artifact','sidecar','transient')),
    UNIQUE (registration_id, seq)
);
CREATE INDEX IF NOT EXISTS idx_atelier_comfy_declared_output_reg ON atelier_comfy_declared_output(registration_id);

CREATE TABLE IF NOT EXISTS atelier_comfy_capability_reject (
    reject_id       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    registration_id UUID NOT NULL REFERENCES atelier_comfy_capability_registration(registration_id) ON DELETE CASCADE,
    seq             BIGINT NOT NULL,
    output_slot     TEXT NOT NULL,
    reason          TEXT NOT NULL,
    UNIQUE (registration_id, seq)
);
CREATE INDEX IF NOT EXISTS idx_atelier_comfy_capability_reject_reg ON atelier_comfy_capability_reject(registration_id);

CREATE TABLE IF NOT EXISTS atelier_comfy_intake_output (
    intake_output_id        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_run_id         UUID NOT NULL,
    node_execution_id       TEXT NOT NULL,
    registration_id         UUID REFERENCES atelier_comfy_capability_registration(registration_id) ON DELETE SET NULL,
    source_node_instance_id TEXT NOT NULL,
    source_output_slot      TEXT NOT NULL,
    media_kind              TEXT NOT NULL CHECK (media_kind IN ('image','mask','latent_preview','video','sidecar_json')),
    mime                    TEXT NOT NULL,
    artifact_ref            TEXT NOT NULL,
    artifact_manifest_ref   TEXT NOT NULL,
    content_hash            TEXT NOT NULL,
    routing_intent          TEXT NOT NULL CHECK (routing_intent IN ('artifact','sidecar','transient')),
    parent_artifact_ref     TEXT,
    prompt_json_ref         TEXT,
    graph_hash              TEXT,
    seed                    BIGINT,
    materialized_at_utc     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (workflow_run_id, content_hash)
);
CREATE INDEX IF NOT EXISTS idx_atelier_comfy_intake_output_run ON atelier_comfy_intake_output(workflow_run_id, materialized_at_utc);
CREATE INDEX IF NOT EXISTS idx_atelier_comfy_intake_output_reg ON atelier_comfy_intake_output(registration_id);

CREATE TABLE IF NOT EXISTS atelier_comfy_fallback_marker (
    workflow_run_id UUID PRIMARY KEY,
    fallback_reason TEXT NOT NULL,
    engaged_at_utc  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

------------------------------------------------------------------------------
-- SOURCING-SPEC + HANDLER VERSION MATRIX (S6.12 / MT-201).
------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS atelier_sourcing_spec (
    record_id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sourcing_spec_id      TEXT NOT NULL,
    schema_version        TEXT NOT NULL,
    source_kind           TEXT NOT NULL,
    source_ref            TEXT NOT NULL,
    handler_family        TEXT NOT NULL,
    handler_version_pin   TEXT NOT NULL,
    params_json           JSONB NOT NULL DEFAULT '{}'::jsonb,
    required_capabilities JSONB NOT NULL DEFAULT '[]'::jsonb,
    idempotency_key       TEXT,
    spec_hash             TEXT NOT NULL UNIQUE,
    created_at_utc        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_atelier_sourcing_spec_id ON atelier_sourcing_spec(sourcing_spec_id);
CREATE INDEX IF NOT EXISTS idx_atelier_sourcing_spec_family ON atelier_sourcing_spec(handler_family);

CREATE TABLE IF NOT EXISTS atelier_handler_version_matrix (
    entry_id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    handler_family        TEXT NOT NULL,
    handler_version       TEXT NOT NULL,
    schema_version_min    TEXT NOT NULL,
    schema_version_max    TEXT NOT NULL,
    side_effect           TEXT NOT NULL CHECK (side_effect IN ('READ','WRITE','EXECUTE')),
    idempotency           TEXT NOT NULL CHECK (idempotency IN ('IDEMPOTENT','IDEMPOTENT_WITH_KEY','NON_IDEMPOTENT')),
    required_capabilities JSONB NOT NULL DEFAULT '[]'::jsonb,
    determinism           TEXT NOT NULL,
    status                TEXT NOT NULL CHECK (status IN ('ACTIVE','DEPRECATED','SUNSET')),
    job_profile_ref       TEXT NOT NULL,
    created_at_utc        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT atelier_handler_version_matrix_family_version_uq UNIQUE (handler_family, handler_version)
);
CREATE INDEX IF NOT EXISTS idx_atelier_handler_version_matrix_family ON atelier_handler_version_matrix(handler_family, created_at_utc DESC);

CREATE TABLE IF NOT EXISTS atelier_sourcing_binding_decision (
    decision_id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sourcing_spec_id         TEXT NOT NULL,
    spec_hash                TEXT NOT NULL,
    handler_family           TEXT NOT NULL,
    handler_version_pin      TEXT NOT NULL,
    bound                    BOOLEAN NOT NULL,
    resolved_handler_version TEXT,
    matched_entry_id         UUID REFERENCES atelier_handler_version_matrix(entry_id) ON DELETE SET NULL,
    matrix_snapshot_id       UUID NOT NULL,
    capability_satisfied     BOOLEAN NOT NULL,
    resolution_reason        TEXT NOT NULL,
    created_at_utc           TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_atelier_binding_decision_spec_hash ON atelier_sourcing_binding_decision(spec_hash);
CREATE INDEX IF NOT EXISTS idx_atelier_binding_decision_matched_entry ON atelier_sourcing_binding_decision(matched_entry_id);

CREATE TABLE IF NOT EXISTS atelier_version_mismatch_receipt (
    receipt_id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    decision_id        UUID NOT NULL UNIQUE REFERENCES atelier_sourcing_binding_decision(decision_id) ON DELETE CASCADE,
    sourcing_spec_id   TEXT NOT NULL,
    spec_hash          TEXT NOT NULL,
    requested_pin      TEXT NOT NULL,
    evaluated_versions JSONB NOT NULL DEFAULT '[]'::jsonb,
    matrix_snapshot_id UUID NOT NULL,
    reason             TEXT NOT NULL CHECK (reason IN ('no_matching_version','schema_unsupported','sunset','capability_denied')),
    created_at_utc     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS atelier_sourcing_ingestion_receipt (
    receipt_id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    decision_id            UUID NOT NULL REFERENCES atelier_sourcing_binding_decision(decision_id) ON DELETE CASCADE,
    ingestion_key          TEXT NOT NULL UNIQUE,
    handler_family         TEXT NOT NULL,
    handler_version        TEXT NOT NULL,
    spec_hash              TEXT NOT NULL,
    artifact_manifest_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
    outcome                TEXT NOT NULL CHECK (outcome IN ('fresh','deduped')),
    completed_count        BIGINT NOT NULL DEFAULT 0,
    pending_count          BIGINT NOT NULL DEFAULT 0,
    created_at_utc         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_atelier_ingestion_receipt_decision ON atelier_sourcing_ingestion_receipt(decision_id);

------------------------------------------------------------------------------
-- MEDIA TRANSCRIPT + CAPTION PIPELINE (S6.11 / MT-203). Governed records only.
------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS atelier_media_probe_report (
    probe_report_id      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    media_source_id      TEXT NOT NULL,
    source_media_hash    TEXT NOT NULL UNIQUE,
    container            TEXT NOT NULL DEFAULT '',
    duration_ms          BIGINT NOT NULL DEFAULT 0,
    streams              JSONB NOT NULL DEFAULT '[]'::jsonb,
    ffprobe_tool_version TEXT NOT NULL DEFAULT '',
    artifact_ref         TEXT NOT NULL,
    probed_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at_utc       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_atelier_media_probe_source ON atelier_media_probe_report(media_source_id);

CREATE TABLE IF NOT EXISTS atelier_transcript_artifact (
    transcript_id     UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    media_source_id   TEXT NOT NULL,
    source_media_hash TEXT NOT NULL REFERENCES atelier_media_probe_report(source_media_hash) ON DELETE RESTRICT,
    language          TEXT NOT NULL DEFAULT '',
    model             JSONB NOT NULL DEFAULT '{}'::jsonb,
    selection_path    TEXT NOT NULL DEFAULT '',
    segments          JSONB NOT NULL DEFAULT '[]'::jsonb,
    timing_anchors    JSONB NOT NULL DEFAULT '[]'::jsonb,
    format_version    TEXT NOT NULL DEFAULT 'TranscriptArtifactV1',
    artifact_ref      TEXT NOT NULL UNIQUE,
    created_at_utc    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_atelier_transcript_hash ON atelier_transcript_artifact(source_media_hash);

CREATE TABLE IF NOT EXISTS atelier_caption_artifact (
    caption_artifact_id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    transcript_id               UUID NOT NULL REFERENCES atelier_transcript_artifact(transcript_id) ON DELETE CASCADE,
    media_source_id             TEXT NOT NULL,
    source_media_hash           TEXT NOT NULL,
    format                      TEXT NOT NULL CHECK (format IN ('srt','vtt','ass')),
    language                    TEXT NOT NULL DEFAULT '',
    max_line_chars              BIGINT NOT NULL DEFAULT 0,
    max_lines_per_cue           BIGINT NOT NULL DEFAULT 0,
    min_cue_ms                  BIGINT NOT NULL DEFAULT 0,
    max_cue_ms                  BIGINT NOT NULL DEFAULT 0,
    cue_count                   BIGINT NOT NULL DEFAULT 0,
    derived_from_timing_anchors BOOLEAN NOT NULL DEFAULT TRUE,
    artifact_ref                TEXT NOT NULL UNIQUE,
    muxed_media_artifact_id     TEXT,
    created_at_utc              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_atelier_caption_transcript ON atelier_caption_artifact(transcript_id);
CREATE INDEX IF NOT EXISTS idx_atelier_caption_hash ON atelier_caption_artifact(source_media_hash);

CREATE TABLE IF NOT EXISTS atelier_transcript_receipt (
    receipt_id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    kind                TEXT NOT NULL CHECK (kind IN ('media_probe','transcribe','caption_render')),
    job_id              TEXT NOT NULL UNIQUE,
    feature_id          TEXT NOT NULL DEFAULT 'FEAT-ASR',
    source_media_hash   TEXT NOT NULL,
    input_artifact_ids  JSONB NOT NULL DEFAULT '[]'::jsonb,
    output_artifact_id  TEXT,
    capability_grants   JSONB NOT NULL DEFAULT '[]'::jsonb,
    tool_versions       JSONB NOT NULL DEFAULT '{}'::jsonb,
    status              TEXT NOT NULL CHECK (status IN ('completed','failed')),
    error_class         TEXT,
    partial_artifact_id TEXT,
    emitted_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at_utc      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_atelier_transcript_receipt_source ON atelier_transcript_receipt(source_media_hash, emitted_at DESC);
CREATE INDEX IF NOT EXISTS idx_atelier_transcript_receipt_kind ON atelier_transcript_receipt(kind, status);

------------------------------------------------------------------------------
-- MEDIA-DOWNLOADER-V2 (S6.10 / MT-204). Governed records; secrets by-ref only.
------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS atelier_md_output_root (
    root_id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    configured_root      TEXT NOT NULL UNIQUE,
    materialization_mode TEXT NOT NULL DEFAULT 'hardlink' CHECK (materialization_mode IN ('copy', 'hardlink', 'symlink')),
    per_mode_subdirs     JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at_utc       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS atelier_md_allowlist_policy (
    allowlist_policy_id  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name                 TEXT NOT NULL UNIQUE,
    allowed_domains      JSONB NOT NULL DEFAULT '[]'::jsonb,
    explicit_url_lists   JSONB NOT NULL DEFAULT '[]'::jsonb,
    default_decision     TEXT NOT NULL DEFAULT 'deny' CHECK (default_decision IN ('deny', 'allow')),
    rate_limit           JSONB NOT NULL DEFAULT '{}'::jsonb,
    max_pages            BIGINT NOT NULL DEFAULT 1500 CHECK (max_pages >= 1 AND max_pages <= 5000),
    robots_posture       TEXT NOT NULL DEFAULT 'respect',
    created_at_utc       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS atelier_md_auth_context (
    auth_context_ref        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    label                   TEXT NOT NULL UNIQUE,
    auth_mode               TEXT NOT NULL DEFAULT 'none' CHECK (auth_mode IN ('none', 'session', 'cookie_jar', 'header')),
    session_ref             TEXT,
    cookie_jar_artifact_ref TEXT,
    header_secret_refs      JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at_utc          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS atelier_md_download_session (
    session_id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    parent_job_id       TEXT NOT NULL,
    idempotency_key     TEXT NOT NULL UNIQUE,
    source_kind         TEXT NOT NULL CHECK (source_kind IN ('youtube', 'instagram', 'forumcrawler', 'videodownloader')),
    auth_context_ref    UUID REFERENCES atelier_md_auth_context(auth_context_ref) ON DELETE SET NULL,
    allowlist_policy_id UUID NOT NULL REFERENCES atelier_md_allowlist_policy(allowlist_policy_id),
    output_root_id      UUID NOT NULL REFERENCES atelier_md_output_root(root_id),
    stage               TEXT NOT NULL DEFAULT 'resolving' CHECK (stage IN ('resolving', 'enqueued', 'fetching', 'probing', 'merging', 'materializing', 'finalized', 'paused', 'failed', 'cancelled')),
    created_at_utc      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_atelier_md_session_stage ON atelier_md_download_session(stage, updated_at_utc DESC);
CREATE INDEX IF NOT EXISTS idx_atelier_md_session_job ON atelier_md_download_session(parent_job_id);

CREATE TABLE IF NOT EXISTS atelier_md_item_state (
    item_id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id       UUID NOT NULL REFERENCES atelier_md_download_session(session_id) ON DELETE CASCADE,
    normalized_url   TEXT NOT NULL,
    stable_source_id TEXT,
    content_hash     TEXT,
    stage            TEXT NOT NULL DEFAULT 'enqueued' CHECK (stage IN ('enqueued', 'fetching', 'probing', 'merging', 'materializing', 'finalized', 'skipped', 'failed')),
    bytes_downloaded BIGINT NOT NULL DEFAULT 0,
    bytes_total      BIGINT,
    part_path_ref    TEXT,
    attempt_count    BIGINT NOT NULL DEFAULT 0,
    last_error_code  TEXT,
    resume_token     TEXT,
    created_at_utc   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (session_id, normalized_url)
);
CREATE INDEX IF NOT EXISTS idx_atelier_md_item_session_stage ON atelier_md_item_state(session_id, stage);
CREATE INDEX IF NOT EXISTS idx_atelier_md_item_stable_id ON atelier_md_item_state(stable_source_id);
CREATE INDEX IF NOT EXISTS idx_atelier_md_item_content_hash ON atelier_md_item_state(content_hash);

CREATE TABLE IF NOT EXISTS atelier_md_checkpoint (
    checkpoint_id    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id       UUID NOT NULL REFERENCES atelier_md_download_session(session_id) ON DELETE CASCADE,
    item_id          UUID REFERENCES atelier_md_item_state(item_id) ON DELETE CASCADE,
    stage            TEXT NOT NULL,
    bytes_downloaded BIGINT NOT NULL DEFAULT 0,
    bytes_total      BIGINT,
    resume_token     TEXT,
    created_at_utc   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_atelier_md_checkpoint_session ON atelier_md_checkpoint(session_id, created_at_utc DESC);
CREATE INDEX IF NOT EXISTS idx_atelier_md_checkpoint_item ON atelier_md_checkpoint(item_id, created_at_utc DESC);

CREATE TABLE IF NOT EXISTS atelier_md_session_receipt (
    receipt_id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id            UUID NOT NULL REFERENCES atelier_md_download_session(session_id) ON DELETE CASCADE,
    parent_job_id         TEXT NOT NULL,
    source_kind           TEXT NOT NULL CHECK (source_kind IN ('youtube', 'instagram', 'forumcrawler', 'videodownloader')),
    auth_context_ref      UUID REFERENCES atelier_md_auth_context(auth_context_ref) ON DELETE SET NULL,
    allowlist_policy_id   UUID NOT NULL REFERENCES atelier_md_allowlist_policy(allowlist_policy_id),
    output_root_id        UUID NOT NULL REFERENCES atelier_md_output_root(root_id),
    item_count            BIGINT NOT NULL DEFAULT 0,
    succeeded             BIGINT NOT NULL DEFAULT 0,
    failed                BIGINT NOT NULL DEFAULT 0,
    skipped_deduped       BIGINT NOT NULL DEFAULT 0,
    materialized_paths    JSONB NOT NULL DEFAULT '[]'::jsonb,
    manifest_artifact_ref TEXT,
    started_at_utc        TIMESTAMPTZ,
    ended_at_utc          TIMESTAMPTZ,
    terminal_stage        TEXT NOT NULL CHECK (terminal_stage IN ('finalized', 'failed', 'cancelled')),
    created_at_utc        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (session_id, terminal_stage)
);
CREATE INDEX IF NOT EXISTS idx_atelier_md_receipt_session ON atelier_md_session_receipt(session_id);

------------------------------------------------------------------------------
-- COMMAND-CORPUS / ACTION-CATALOG PARITY (S10.19 / MT-206). Projection only.
------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS atelier_command_corpus_entry (
    entry_id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    action_id            TEXT NOT NULL UNIQUE,
    corpus_source        TEXT NOT NULL CHECK (corpus_source IN ('preload', 'ipc_handler', 'both')),
    owner                TEXT NOT NULL,
    actor_eligibility    JSONB NOT NULL DEFAULT '[]'::jsonb,
    params_schema_ref    TEXT NOT NULL,
    input_schema_version INTEGER NOT NULL DEFAULT 1,
    capabilities         JSONB NOT NULL DEFAULT '[]'::jsonb,
    execution_class      TEXT NOT NULL CHECK (execution_class IN ('pure_projection', 'write_box', 'workflow_job', 'ai_job')),
    receipt_shape        TEXT NOT NULL,
    errors               JSONB NOT NULL DEFAULT '[]'::jsonb,
    foreground_flag      BOOLEAN NOT NULL DEFAULT FALSE,
    manual_anchor        TEXT NOT NULL,
    evidence_class       JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at_utc       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_atelier_command_corpus_entry_owner ON atelier_command_corpus_entry(owner, action_id);
CREATE INDEX IF NOT EXISTS idx_atelier_command_corpus_entry_manual_anchor ON atelier_command_corpus_entry(manual_anchor);

CREATE TABLE IF NOT EXISTS atelier_command_corpus_blocked (
    blocked_id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    action_id            TEXT NOT NULL,
    blocked_reason       TEXT NOT NULL CHECK (blocked_reason IN (
                             'no_manual_anchor', 'ungoverned_execution', 'no_capability_gate',
                             'foreground_unbounded', 'no_typed_receipt', 'no_event_evidence')),
    discovered_in        TEXT NOT NULL CHECK (discovered_in IN ('preload', 'ipc_handler', 'both')),
    recovery_instruction TEXT NOT NULL,
    first_seen_utc       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_utc        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (action_id, blocked_reason)
);
CREATE INDEX IF NOT EXISTS idx_atelier_command_corpus_blocked_action ON atelier_command_corpus_blocked(action_id);
CREATE INDEX IF NOT EXISTS idx_atelier_command_corpus_blocked_last_seen ON atelier_command_corpus_blocked(last_seen_utc DESC, action_id ASC);

CREATE TABLE IF NOT EXISTS atelier_command_corpus_parity_report (
    report_id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    total_corpus          BIGINT NOT NULL DEFAULT 0,
    covered_count         BIGINT NOT NULL DEFAULT 0,
    blocked_count         BIGINT NOT NULL DEFAULT 0,
    orphaned_manual_count BIGINT NOT NULL DEFAULT 0,
    defects               JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at_utc        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_atelier_command_corpus_parity_report_created ON atelier_command_corpus_parity_report(created_at_utc DESC, report_id DESC);

------------------------------------------------------------------------------
-- STEALTH REFERENCE WINDOW (S10.18 / MT-205). State + read-only projection.
------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS atelier_stealth_window (
    window_ref_id  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_actor    TEXT NOT NULL,
    title          TEXT NOT NULL,
    visibility     TEXT NOT NULL CHECK (visibility IN ('off_screen_only', 'diagnostic_embed', 'foreground_exception_bound')),
    quiet_json     JSONB NOT NULL DEFAULT '{}'::jsonb,
    layout_json    JSONB NOT NULL DEFAULT '{}'::jsonb,
    status         TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'closed')),
    revision       BIGINT NOT NULL DEFAULT 1,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT atelier_stealth_window_owner_title_uq UNIQUE (owner_actor, title)
);
CREATE INDEX IF NOT EXISTS idx_atelier_stealth_window_owner ON atelier_stealth_window(owner_actor, updated_at_utc DESC);
CREATE INDEX IF NOT EXISTS idx_atelier_stealth_window_owner_status ON atelier_stealth_window(owner_actor, status);

CREATE TABLE IF NOT EXISTS atelier_stealth_ref (
    ref_id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    window_ref_id   UUID NOT NULL REFERENCES atelier_stealth_window(window_ref_id) ON DELETE CASCADE,
    seq             BIGINT NOT NULL,
    ref_kind        TEXT NOT NULL CHECK (ref_kind IN ('artifact', 'spec_anchor', 'transcript', 'job_receipt', 'ledger_event', 'screenshot', 'diagnostic')),
    resolver        TEXT NOT NULL,
    content_sha256  TEXT NOT NULL,
    redaction_state BOOLEAN NOT NULL,
    pinned_at_utc   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT atelier_stealth_ref_window_seq_uq UNIQUE (window_ref_id, seq)
);
CREATE INDEX IF NOT EXISTS idx_atelier_stealth_ref_window_seq ON atelier_stealth_ref(window_ref_id, seq);

CREATE TABLE IF NOT EXISTS atelier_stealth_capture (
    capture_id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    window_ref_id        UUID NOT NULL REFERENCES atelier_stealth_window(window_ref_id) ON DELETE CASCADE,
    artifact_manifest_id TEXT NOT NULL,
    content_sha256       TEXT NOT NULL,
    captured_at_utc      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT atelier_stealth_capture_window_manifest_uq UNIQUE (window_ref_id, artifact_manifest_id)
);
CREATE INDEX IF NOT EXISTS idx_atelier_stealth_capture_window ON atelier_stealth_capture(window_ref_id, captured_at_utc DESC);
