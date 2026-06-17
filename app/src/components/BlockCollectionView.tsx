import { useCallback, useEffect, useState } from "react";

import {
  BLOCK_VIEW_UNTAGGED_LANE,
  getBlockView,
  queryBlockViewResults,
  updateBlockView,
  updateLoomBlock,
  type BlockViewDefinition,
  type BlockViewField,
  type BlockViewResults,
  type LoomBlock,
} from "../lib/api";
import { LoomCalendarView } from "./LoomCalendarView";
import { LoomKanbanView } from "./LoomKanbanView";
import { LoomTableView } from "./LoomTableView";

/**
 * MT-262 BlockCollectionView — the saved-view host. A saved view is a typed
 * `LoomBlock(content_type='view_def')`; its definition lives in PostgreSQL. This
 * host:
 *  - loads the view definition + executes its query against the REAL Loom query
 *    backend (`queryBlockViewResults`, all filter/sort/group SQL-side);
 *  - on a table header click, PERSISTS the new sort (`updateBlockView`) and
 *    re-queries (never client-side sort over a partial page);
 *  - on a Kanban card move, performs the REAL tag edge delete+create via
 *    `updateLoomBlock` (with receipts), then re-queries (never local state).
 *
 * Authority is PostgreSQL + EventLedger; this component is a projection.
 */

export type BlockCollectionViewProps = {
  workspaceId: string;
  viewBlockId: string;
  /** Page size for the underlying query. */
  limit?: number;
};

function flipDirection(
  definition: BlockViewDefinition,
  field: BlockViewField,
): BlockViewDefinition {
  const current = definition.sort;
  const sameField = current?.field === field;
  const nextDirection =
    sameField && current?.direction === "asc" ? "desc" : sameField ? "asc" : "asc";
  return { ...definition, sort: { field, direction: nextDirection } };
}

export function BlockCollectionView({
  workspaceId,
  viewBlockId,
  limit = 100,
}: BlockCollectionViewProps) {
  const [definition, setDefinition] = useState<BlockViewDefinition | null>(null);
  const [results, setResults] = useState<BlockViewResults | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<string>("");

  const runQuery = useCallback(async () => {
    try {
      const next = await queryBlockViewResults(workspaceId, viewBlockId, { limit });
      setResults(next);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [workspaceId, viewBlockId, limit]);

  useEffect(() => {
    let cancelled = false;
    const load = async () => {
      try {
        const record = await getBlockView(workspaceId, viewBlockId);
        if (cancelled) return;
        setDefinition(record.definition);
        const next = await queryBlockViewResults(workspaceId, viewBlockId, { limit });
        if (cancelled) return;
        setResults(next);
        setError(null);
      } catch (err) {
        if (cancelled) return;
        setError(err instanceof Error ? err.message : String(err));
      }
    };
    void load();
    return () => {
      cancelled = true;
    };
  }, [workspaceId, viewBlockId, limit]);

  // Table header click -> persist new sort + re-query (SQL-side ORDER BY).
  const handleSortChange = useCallback(
    async (field: BlockViewField) => {
      if (!definition) return;
      const next = flipDirection(definition, field);
      setStatus("Re-sorting…");
      try {
        const record = await updateBlockView(workspaceId, viewBlockId, next);
        setDefinition(record.definition);
        await runQuery();
        setStatus("");
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      }
    },
    [definition, workspaceId, viewBlockId, runQuery],
  );

  // Kanban card move -> REAL tag edge mutation (with receipt) + re-query.
  const handleCardMove = useCallback(
    async (block: LoomBlock, fromKey: string, toKey: string) => {
      setStatus("Moving card…");
      const add_tags = toKey === BLOCK_VIEW_UNTAGGED_LANE ? [] : [toKey];
      const remove_tags = fromKey === BLOCK_VIEW_UNTAGGED_LANE ? [] : [fromKey];
      try {
        await updateLoomBlock(workspaceId, block.block_id, { add_tags, remove_tags });
        // Re-query the backend so the lane membership reflects authority, not
        // any local state mutation.
        await runQuery();
        setStatus("");
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      }
    },
    [workspaceId, runQuery],
  );

  if (error) {
    return (
      <div className="block-collection-view" data-testid="block-collection-view">
        <p role="alert" data-testid="block-collection-view-error">
          {error}
        </p>
      </div>
    );
  }

  if (!definition || !results) {
    return (
      <div className="block-collection-view" data-testid="block-collection-view">
        <p data-testid="block-collection-view-loading">Loading view…</p>
      </div>
    );
  }

  return (
    <div
      className="block-collection-view"
      data-testid="block-collection-view"
      data-view-kind={definition.kind}
    >
      {status ? <p data-testid="block-collection-view-status">{status}</p> : null}
      {definition.kind === "table" ? (
        <LoomTableView
          blocks={results.blocks}
          columns={definition.columns ?? []}
          sort={definition.sort ?? null}
          onSortChange={(field) => void handleSortChange(field)}
        />
      ) : null}
      {definition.kind === "kanban" ? (
        <LoomKanbanView lanes={results.groups ?? []} onCardMove={handleCardMove} />
      ) : null}
      {definition.kind === "calendar" ? (
        <LoomCalendarView
          blocks={results.blocks}
          dateField={definition.calendar_date_field ?? null}
        />
      ) : null}
    </div>
  );
}
