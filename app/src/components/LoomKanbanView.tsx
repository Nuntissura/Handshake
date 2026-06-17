import { useState, type DragEvent } from "react";

import { BLOCK_VIEW_UNTAGGED_LANE, type BlockViewLane, type LoomBlock } from "../lib/api";

/**
 * MT-262 LoomKanbanView — a tag-grouped Kanban over REAL Loom query results.
 *
 * Dragging a card between lanes is a REAL mutation: `onCardMove` deletes the
 * card's old tag edge and creates the new one (via the normal block/edge update
 * paths, each leaving a receipt), THEN the parent re-queries the backend. This
 * component never treats its local lane arrays as truth — after a drop it asks
 * the parent to persist + re-query.
 */

export type LoomKanbanViewProps = {
  lanes: BlockViewLane[];
  /**
   * Move `block` from lane `fromKey` to lane `toKey`. The parent performs the
   * real tag edge delete+create and re-queries. Returns when persistence is
   * done so the UI can clear its in-flight state.
   */
  onCardMove: (block: LoomBlock, fromKey: string, toKey: string) => Promise<void>;
};

const DRAG_MIME = "application/x-handshake-kanban-card";

type DragState = { blockId: string; fromKey: string } | null;

function laneLabel(key: string): string {
  return key === BLOCK_VIEW_UNTAGGED_LANE ? "Untagged" : key;
}

export function LoomKanbanView({ lanes, onCardMove }: LoomKanbanViewProps) {
  const [drag, setDrag] = useState<DragState>(null);
  const [busy, setBusy] = useState(false);

  const handleDrop = async (toKey: string, event: DragEvent<HTMLDivElement>) => {
    event.preventDefault();
    const raw = event.dataTransfer.getData(DRAG_MIME);
    const state: DragState = raw ? (JSON.parse(raw) as DragState) : drag;
    setDrag(null);
    if (!state || state.fromKey === toKey || busy) return;
    const lane = lanes.find((l) => l.key === state.fromKey);
    const block = lane?.blocks.find((b) => b.block_id === state.blockId);
    if (!block) return;
    setBusy(true);
    try {
      // Real mutation + re-query happen in the parent; never local state.
      await onCardMove(block, state.fromKey, toKey);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="loom-kanban-view" data-testid="loom-kanban-view" data-busy={busy}>
      {lanes.map((lane) => (
        <div
          key={lane.key}
          className="loom-kanban-lane"
          data-testid="loom-kanban-lane"
          data-lane-key={lane.key}
          onDragOver={(event) => event.preventDefault()}
          onDrop={(event) => void handleDrop(lane.key, event)}
        >
          <header className="loom-kanban-lane-header">{laneLabel(lane.key)}</header>
          <ul>
            {lane.blocks.map((block) => (
              <li
                key={block.block_id}
                className="loom-kanban-card"
                data-testid="loom-kanban-card"
                data-block-id={block.block_id}
                data-lane-key={lane.key}
                draggable
                onDragStart={(event) => {
                  const payload = { blockId: block.block_id, fromKey: lane.key };
                  event.dataTransfer.setData(DRAG_MIME, JSON.stringify(payload));
                  setDrag(payload);
                }}
              >
                {block.title ?? block.block_id}
              </li>
            ))}
          </ul>
        </div>
      ))}
    </div>
  );
}
