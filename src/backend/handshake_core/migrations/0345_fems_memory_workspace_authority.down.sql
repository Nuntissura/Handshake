DROP INDEX IF EXISTS idx_fems_memory_proposals_ws_request;

ALTER TABLE IF EXISTS fems_memory_packs
    DROP CONSTRAINT IF EXISTS fk_fems_memory_packs_workspace;
ALTER TABLE IF EXISTS fems_memory_proposals
    DROP CONSTRAINT IF EXISTS fk_fems_memory_proposals_workspace;
ALTER TABLE IF EXISTS fems_memory_items
    DROP CONSTRAINT IF EXISTS fk_fems_memory_items_workspace;

-- Keep the request identity in the proposal JSON across a rollback. The pre-0345
-- reader treats proposal JSON as opaque, while a later re-upgrade can recover the
-- exact explicit identity instead of deriving a different one and breaking retries.
ALTER TABLE IF EXISTS fems_memory_proposals DROP COLUMN IF EXISTS request_id;
