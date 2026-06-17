import type { BlockViewField, LoomBlock } from "../lib/api";

/**
 * MT-262 LoomCalendarView — buckets REAL Loom query results by a date field
 * (journal_date or created/updated). The date window is applied SERVER-SIDE
 * (the parent sends date_from/date_to to the backend); this component only
 * groups the rows it was given by their real date value. Projection only.
 */

export type LoomCalendarViewProps = {
  blocks: LoomBlock[];
  /** Which real date field to bucket by (defaults to created). */
  dateField: BlockViewField | null;
};

function bucketKey(block: LoomBlock, field: BlockViewField | null): string {
  switch (field) {
    case "journal_date":
      return block.journal_date ?? "undated";
    case "updated":
      return block.updated_at.slice(0, 10);
    case "created":
    default:
      return block.created_at.slice(0, 10);
  }
}

export function LoomCalendarView({ blocks, dateField }: LoomCalendarViewProps) {
  const buckets = new Map<string, LoomBlock[]>();
  for (const block of blocks) {
    const key = bucketKey(block, dateField);
    const list = buckets.get(key) ?? [];
    list.push(block);
    buckets.set(key, list);
  }
  const orderedKeys = Array.from(buckets.keys()).sort();

  return (
    <div className="loom-calendar-view" data-testid="loom-calendar-view">
      {orderedKeys.map((key) => (
        <div
          key={key}
          className="loom-calendar-day"
          data-testid="loom-calendar-day"
          data-date={key}
        >
          <header className="loom-calendar-day-header">{key}</header>
          <ul>
            {(buckets.get(key) ?? []).map((block) => (
              <li
                key={block.block_id}
                data-testid="loom-calendar-entry"
                data-block-id={block.block_id}
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
