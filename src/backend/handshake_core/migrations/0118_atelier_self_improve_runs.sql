-- WP-KERNEL-005 MT-151 validator-first-pass-in-sandbox run records.
-- Every self-improvement-loop sandbox provisioning run and every validator
-- first-pass execution persists a durable row so evaluator metrics are
-- backed by re-readable PostgreSQL evidence instead of in-memory counters.

CREATE TABLE IF NOT EXISTS atelier_self_improve_sandbox_run (
    sandbox_run_id     UUID PRIMARY KEY,
    surface_kind       TEXT NOT NULL,
    snapshot_sha256    TEXT NOT NULL,
    workspace_ref      TEXT NOT NULL,
    status             TEXT NOT NULL,
    started_at_utc     TIMESTAMPTZ NOT NULL,
    completed_at_utc   TIMESTAMPTZ NOT NULL,
    created_at_utc     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_si_sandbox_run_surface_kind
        CHECK (surface_kind IN ('model_manual', 'retrieval_policy')),
    CONSTRAINT chk_atelier_si_sandbox_run_snapshot_sha256
        CHECK (snapshot_sha256 ~ '^sha256:[0-9a-f]{64}$'),
    CONSTRAINT chk_atelier_si_sandbox_run_workspace_ref_trimmed
        CHECK (workspace_ref = btrim(workspace_ref) AND workspace_ref <> ''),
    CONSTRAINT chk_atelier_si_sandbox_run_status
        CHECK (status IN ('provisioned', 'failed')),
    CONSTRAINT chk_atelier_si_sandbox_run_timing
        CHECK (completed_at_utc >= started_at_utc)
);

CREATE TABLE IF NOT EXISTS atelier_validator_first_pass_run (
    first_pass_run_id  UUID PRIMARY KEY,
    sandbox_run_id     UUID REFERENCES atelier_self_improve_sandbox_run(sandbox_run_id),
    corpus_item_id     UUID NOT NULL,
    hbr_rule_id        TEXT NOT NULL,
    packet_under_test  TEXT NOT NULL,
    transition         TEXT NOT NULL,
    verdict            TEXT NOT NULL,
    failing_rule_count INTEGER NOT NULL DEFAULT 0,
    latency_ms         BIGINT NOT NULL,
    capsule_bytes      BIGINT NOT NULL,
    gate_event_id      UUID,
    created_at_utc     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_vfp_run_hbr_rule_trimmed
        CHECK (hbr_rule_id = btrim(hbr_rule_id) AND hbr_rule_id <> ''),
    CONSTRAINT chk_atelier_vfp_run_packet_trimmed
        CHECK (packet_under_test = btrim(packet_under_test)
               AND packet_under_test <> ''),
    CONSTRAINT chk_atelier_vfp_run_transition_trimmed
        CHECK (transition = btrim(transition) AND transition <> ''),
    CONSTRAINT chk_atelier_vfp_run_verdict
        CHECK (verdict IN ('pass', 'fail', 'skip')),
    CONSTRAINT chk_atelier_vfp_run_failing_count
        CHECK (failing_rule_count >= 0),
    CONSTRAINT chk_atelier_vfp_run_latency
        CHECK (latency_ms >= 0),
    CONSTRAINT chk_atelier_vfp_run_capsule_bytes
        CHECK (capsule_bytes >= 0)
);

CREATE INDEX IF NOT EXISTS idx_atelier_vfp_run_sandbox
    ON atelier_validator_first_pass_run(sandbox_run_id, created_at_utc DESC);

CREATE INDEX IF NOT EXISTS idx_atelier_vfp_run_corpus_item
    ON atelier_validator_first_pass_run(corpus_item_id);
