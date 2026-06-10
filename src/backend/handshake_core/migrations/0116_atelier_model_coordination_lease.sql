-- WP-KERNEL-005 MT-143 lease/claim contract for parallel model coordination.
-- Leases are persisted rows; TTL expiry is computed against the database
-- clock on every read (lease_expires_at_utc vs NOW()), so stale state is
-- observable on re-read without a writer. Exclusive leases and handoff
-- reservations admit one active unexpired claimant per thread; conflicting
-- claims are rejected in the claim transaction.

CREATE TABLE IF NOT EXISTS atelier_model_coordination_lease (
    claim_id              UUID PRIMARY KEY,
    thread_id             TEXT NOT NULL,
    executor_kind         TEXT NOT NULL,
    actor_id              TEXT NOT NULL,
    session_id            TEXT NOT NULL,
    claim_mode            TEXT NOT NULL,
    lease_state           TEXT NOT NULL DEFAULT 'active',
    claimed_at_utc        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ttl_seconds           BIGINT NOT NULL,
    lease_expires_at_utc  TIMESTAMPTZ NOT NULL,
    released_at_utc       TIMESTAMPTZ,
    taken_over_at_utc     TIMESTAMPTZ,
    takeover_reason       TEXT,
    prior_claim_id        UUID REFERENCES atelier_model_coordination_lease(claim_id),
    linked_work_packet_id TEXT NOT NULL,
    linked_micro_task_id  TEXT NOT NULL,
    created_at_utc        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_model_lease_thread_id_trimmed
        CHECK (thread_id = btrim(thread_id) AND thread_id <> ''),
    CONSTRAINT chk_atelier_model_lease_actor_id_trimmed
        CHECK (actor_id = btrim(actor_id) AND actor_id <> ''),
    CONSTRAINT chk_atelier_model_lease_session_id_trimmed
        CHECK (session_id = btrim(session_id) AND session_id <> ''),
    CONSTRAINT chk_atelier_model_lease_executor_kind
        CHECK (executor_kind IN (
            'local_small_model', 'local_large_model', 'cloud_model',
            'reviewer', 'validator', 'operator', 'workflow_automation'
        )),
    CONSTRAINT chk_atelier_model_lease_claim_mode
        CHECK (claim_mode IN (
            'exclusive_lease', 'shared_observer',
            'broadcast_request', 'handoff_reservation'
        )),
    CONSTRAINT chk_atelier_model_lease_state
        CHECK (lease_state IN ('active', 'released', 'expired', 'taken_over')),
    CONSTRAINT chk_atelier_model_lease_ttl_positive
        CHECK (ttl_seconds > 0),
    CONSTRAINT chk_atelier_model_lease_expiry_after_claim
        CHECK (lease_expires_at_utc > claimed_at_utc),
    CONSTRAINT chk_atelier_model_lease_wp_trimmed
        CHECK (linked_work_packet_id = btrim(linked_work_packet_id)
               AND linked_work_packet_id <> ''),
    CONSTRAINT chk_atelier_model_lease_mt_trimmed
        CHECK (linked_micro_task_id = btrim(linked_micro_task_id)
               AND linked_micro_task_id <> ''),
    CONSTRAINT chk_atelier_model_lease_released_state
        CHECK (released_at_utc IS NULL OR lease_state = 'released'),
    CONSTRAINT chk_atelier_model_lease_takeover_state
        CHECK (taken_over_at_utc IS NULL OR lease_state = 'taken_over')
);

CREATE INDEX IF NOT EXISTS idx_atelier_model_lease_thread_active
    ON atelier_model_coordination_lease(thread_id)
    WHERE lease_state = 'active';

CREATE INDEX IF NOT EXISTS idx_atelier_model_lease_actor
    ON atelier_model_coordination_lease(actor_id, created_at_utc DESC);
