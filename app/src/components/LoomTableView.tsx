import type { BlockViewField, BlockViewSort, BlockViewSortDirection, LoomBlock } from "../lib/api";

/**
 * MT-262 LoomTableView — a sortable typed-column table over REAL Loom query
 * results. A header click does NOT sort client-side: it calls `onSortChange`,
 * which re-queries the backend (the typed ORDER BY runs in SQL). This component
 * only renders the rows it was given; it is a projection.
 */

export type LoomTableViewProps = {
  blocks: LoomBlock[];
  columns: BlockViewField[];
  sort: BlockViewSort | null;
  /** Header click -> re-query the backend with the new typed sort. */
  onSortChange: (field: BlockViewField) => void;
};

const COLUMN_LABELS: Record<BlockViewField, string> = {
  title: "Title",
  created: "Created",
  updated: "Updated",
  journal_date: "Journal date",
  content_type: "Type",
  pinned: "Pinned",
  favorite: "Favorite",
  backlink_count: "Backlinks",
  mention_count: "Mentions",
  tag_count: "Tags",
};

function cellValue(block: LoomBlock, field: BlockViewField): string {
  switch (field) {
    case "title":
      return block.title ?? block.original_filename ?? block.block_id;
    case "created":
      return block.created_at;
    case "updated":
      return block.updated_at;
    case "journal_date":
      return block.journal_date ?? "";
    case "content_type":
      return block.content_type;
    case "pinned":
      return block.pinned ? "yes" : "no";
    case "favorite":
      return block.favorite ? "yes" : "no";
    case "backlink_count":
      return String(block.derived.backlink_count ?? 0);
    case "mention_count":
      return String(block.derived.mention_count ?? 0);
    case "tag_count":
      return String(block.derived.tag_count ?? 0);
    default:
      return "";
  }
}

function sortIndicator(
  field: BlockViewField,
  sort: BlockViewSort | null,
): "" | BlockViewSortDirection {
  if (!sort || sort.field !== field) return "";
  return sort.direction ?? "desc";
}

export function LoomTableView({ blocks, columns, sort, onSortChange }: LoomTableViewProps) {
  const cols = columns.length > 0 ? columns : (["title", "updated"] as BlockViewField[]);
  return (
    <table className="loom-table-view" data-testid="loom-table-view">
      <thead>
        <tr>
          {cols.map((field) => {
            const indicator = sortIndicator(field, sort);
            return (
              <th key={field}>
                <button
                  type="button"
                  data-testid={`loom-table-sort-${field}`}
                  data-sort-direction={indicator}
                  onClick={() => onSortChange(field)}
                >
                  {COLUMN_LABELS[field]}
                  {indicator === "asc" ? " ▲" : indicator === "desc" ? " ▼" : ""}
                </button>
              </th>
            );
          })}
        </tr>
      </thead>
      <tbody>
        {blocks.map((block) => (
          <tr key={block.block_id} data-testid="loom-table-row" data-block-id={block.block_id}>
            {cols.map((field) => (
              <td key={field} data-field={field}>
                {cellValue(block, field)}
              </td>
            ))}
          </tr>
        ))}
      </tbody>
    </table>
  );
}
