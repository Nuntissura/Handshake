-- WP-KERNEL-009 MT-261 UnifiedWorkSurface-261-CanvasBoard.
-- Master Spec §7.1.4.3 / §10.12: an Obsidian-canvas-class surface over LoomBlock
-- authority. The canvas itself IS a typed LoomBlock (content_type='canvas');
-- placed items are block-id REFERENCES (FK, never content copies); semantic
-- edges are real loom_edges; visual-only edges are board-local decoration that
-- is explicitly NOT graph authority. Board state (viewport) is JSONB on the
-- canvas row, mirroring the 0323 workbench-layout-state precedent (jsonb CHECK
-- schema_id + event_ledger_event_id FK). Authority is PostgreSQL + EventLedger;
-- the React canvas is a projection only.
--
-- TRAP GUARD: this is the NEW LoomBoard, NOT the legacy Excalidraw sketch canvas
-- (tables canvas_nodes/canvas_edges, migration 0005). Those store content COPIES
-- in `data jsonb`; this table set stores REFERENCES (FK to loom_blocks).

CREATE TABLE IF NOT EXISTS loom_canvas_boards (
    -- The board IS a LoomBlock(content_type='canvas'). PK == block_id; deleting
    -- the canvas block CASCADEs the board row (and its placements/visual edges)
    -- but NEVER the referenced blocks (those FK with ON DELETE RESTRICT below,
    -- and a board row deletion does not touch loom_blocks rows it references).
    block_id TEXT PRIMARY KEY
        REFERENCES loom_blocks(block_id) ON UPDATE RESTRICT ON DELETE CASCADE,
    workspace_id TEXT NOT NULL
        REFERENCES workspaces(id) ON UPDATE RESTRICT ON DELETE CASCADE,
    -- Viewport / board-level state. {schema_id:'hsk.loom_canvas_board@1', pan_x,
    -- pan_y, zoom}. Mirrors 0323 jsonb CHECK + registry pattern.
    board_state JSONB NOT NULL CHECK (
        jsonb_typeof(board_state) = 'object'
        AND board_state ->> 'schema_id' = 'hsk.loom_canvas_board@1'
    ),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    event_ledger_event_id TEXT NOT NULL
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_loom_canvas_boards_workspace
    ON loom_canvas_boards (workspace_id);
CREATE INDEX IF NOT EXISTS idx_loom_canvas_boards_event
    ON loom_canvas_boards (event_ledger_event_id);

-- A placement is a REFERENCE to a LoomBlock positioned on a canvas. It NEVER
-- copies content. ON DELETE RESTRICT on placed_block_id proves the canvas can
-- never silently destroy a referenced block: removing a block requires removing
-- its placements first (the API removes placements; it never copies content).
CREATE TABLE IF NOT EXISTS loom_canvas_placements (
    placement_id TEXT PRIMARY KEY
        CHECK (placement_id ~ '^LCP-[0-9a-f]{32}$'),
    canvas_block_id TEXT NOT NULL
        REFERENCES loom_canvas_boards(block_id) ON UPDATE RESTRICT ON DELETE CASCADE,
    workspace_id TEXT NOT NULL
        REFERENCES workspaces(id) ON UPDATE RESTRICT ON DELETE CASCADE,
    -- The placed block is a pure reference (FK). RESTRICT so a block referenced
    -- on a canvas cannot be deleted out from under the placement.
    placed_block_id TEXT NOT NULL
        REFERENCES loom_blocks(block_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    x DOUBLE PRECISION NOT NULL,
    y DOUBLE PRECISION NOT NULL,
    w DOUBLE PRECISION NOT NULL CHECK (w > 0),
    h DOUBLE PRECISION NOT NULL CHECK (h > 0),
    z_index INTEGER NOT NULL DEFAULT 0,
    group_id TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- A block appears at most once on a given canvas (reference, not copy).
    CONSTRAINT uq_loom_canvas_placement UNIQUE (canvas_block_id, placed_block_id)
);

CREATE INDEX IF NOT EXISTS idx_loom_canvas_placements_canvas
    ON loom_canvas_placements (canvas_block_id, z_index);
CREATE INDEX IF NOT EXISTS idx_loom_canvas_placements_placed
    ON loom_canvas_placements (placed_block_id);

-- A visual-only edge is board-local decoration between two placements. It is
-- EXPLICITLY NOT graph authority: it never becomes a loom_edge and never appears
-- in the local/global Loom graph. Semantic connections use real loom_edges via
-- the existing create_loom_edge path.
CREATE TABLE IF NOT EXISTS loom_canvas_visual_edges (
    visual_edge_id TEXT PRIMARY KEY
        CHECK (visual_edge_id ~ '^LCV-[0-9a-f]{32}$'),
    canvas_block_id TEXT NOT NULL
        REFERENCES loom_canvas_boards(block_id) ON UPDATE RESTRICT ON DELETE CASCADE,
    workspace_id TEXT NOT NULL
        REFERENCES workspaces(id) ON UPDATE RESTRICT ON DELETE CASCADE,
    from_placement_id TEXT NOT NULL
        REFERENCES loom_canvas_placements(placement_id) ON UPDATE RESTRICT ON DELETE CASCADE,
    to_placement_id TEXT NOT NULL
        REFERENCES loom_canvas_placements(placement_id) ON UPDATE RESTRICT ON DELETE CASCADE,
    label TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_loom_canvas_visual_edge_distinct
        CHECK (from_placement_id <> to_placement_id)
);

CREATE INDEX IF NOT EXISTS idx_loom_canvas_visual_edges_canvas
    ON loom_canvas_visual_edges (canvas_block_id);

-- Widen the loom_blocks content_type allow-list to admit the canvas board kind.
-- The constraint name follows the 0005-era loom_blocks definition; drop the
-- existing CHECK and re-add the superset (additive; mirrors the 0333/0290
-- allow-list-widening pattern).
ALTER TABLE loom_blocks
    DROP CONSTRAINT IF EXISTS loom_blocks_content_type_check;

ALTER TABLE loom_blocks
    ADD CONSTRAINT loom_blocks_content_type_check
    CHECK (content_type IN (
        'note',
        'file',
        'annotated_file',
        'tag_hub',
        'journal',
        'canvas'
    ));

-- NOTE: loom_canvas_* belong to the Loom domain (siblings of loom_blocks /
-- loom_edges), which are intentionally NOT registered in
-- knowledge_schema_registry (that registry is the `knowledge_`-prefixed
-- namespace boundary, per chk_knowledge_schema_registry_prefix). Authority of
-- these rows is still PostgreSQL + EventLedger.
