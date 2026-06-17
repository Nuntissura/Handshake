// WP-KERNEL-009 / MT-262 — BlockCollectionView wiring tests.
//
// Mounts the saved-view host in jsdom with the api module mocked and proves:
//   - a table header click PERSISTS the new sort (updateBlockView) and
//     re-queries the backend (never a client-side sort),
//   - a Kanban card move performs the REAL tag edge mutation (updateLoomBlock
//     with add_tags/remove_tags) THEN re-queries (never local state as truth),
//   - the calendar kind buckets rows by their real date field.

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { BlockCollectionView } from "./BlockCollectionView";
import {
  getBlockView,
  queryBlockViewResults,
  updateBlockView,
  updateLoomBlock,
  type BlockViewRecord,
  type BlockViewResults,
  type LoomBlock,
} from "../lib/api";

vi.mock("../lib/api", async () => {
  const actual = await vi.importActual<typeof import("../lib/api")>("../lib/api");
  return {
    ...actual,
    getBlockView: vi.fn(),
    queryBlockViewResults: vi.fn(),
    updateBlockView: vi.fn(),
    updateLoomBlock: vi.fn(),
  };
});

const WS = "ws-mt262";
const VIEW = "view-blk-1";

function block(id: string, title: string, extra: Partial<LoomBlock> = {}): LoomBlock {
  return {
    block_id: id,
    workspace_id: WS,
    content_type: "note",
    title,
    pinned: false,
    favorite: false,
    created_at: "2026-06-10T00:00:00Z",
    updated_at: "2026-06-10T00:00:00Z",
    derived: { backlink_count: 0, mention_count: 0, tag_count: 0, preview_status: "none" },
    ...extra,
  };
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe("BlockCollectionView table", () => {
  it("re-queries the backend with a persisted typed sort on a header click", async () => {
    const tableRecord: BlockViewRecord = {
      block: block(VIEW, "All notes"),
      definition: {
        kind: "table",
        columns: ["title", "updated"],
        sort: { field: "title", direction: "asc" },
      },
    };
    const resorted: BlockViewRecord = {
      block: tableRecord.block,
      definition: {
        kind: "table",
        columns: ["title", "updated"],
        sort: { field: "updated", direction: "asc" },
      },
    };
    const results: BlockViewResults = {
      kind: "table",
      blocks: [block("a", "Alpha"), block("b", "Beta")],
      total_returned: 2,
    };

    vi.mocked(getBlockView).mockResolvedValue(tableRecord);
    vi.mocked(queryBlockViewResults).mockResolvedValue(results);
    vi.mocked(updateBlockView).mockResolvedValue(resorted);

    render(<BlockCollectionView workspaceId={WS} viewBlockId={VIEW} />);

    await waitFor(() => expect(screen.getByTestId("loom-table-view")).toBeTruthy());
    expect(vi.mocked(queryBlockViewResults)).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByTestId("loom-table-sort-updated"));

    await waitFor(() =>
      expect(vi.mocked(updateBlockView)).toHaveBeenCalledWith(
        WS,
        VIEW,
        expect.objectContaining({ sort: { field: "updated", direction: "asc" } }),
      ),
    );
    // The re-sort triggered a SECOND backend query (SQL-side ORDER BY), not a
    // client-side reorder.
    await waitFor(() => expect(vi.mocked(queryBlockViewResults)).toHaveBeenCalledTimes(2));
  });
});

describe("BlockCollectionView kanban", () => {
  it("a card move performs the real tag mutation then re-queries", async () => {
    const kanbanRecord: BlockViewRecord = {
      block: block(VIEW, "Board"),
      definition: {
        kind: "kanban",
        group_by: { kind: "tag" },
        query: { tag_ids: ["tag-todo", "tag-done"] },
      },
    };
    const before: BlockViewResults = {
      kind: "kanban",
      blocks: [],
      total_returned: 1,
      groups: [
        { key: "tag-todo", blocks: [block("card-1", "Ship it")] },
        { key: "tag-done", blocks: [] },
      ],
    };
    const after: BlockViewResults = {
      kind: "kanban",
      blocks: [],
      total_returned: 1,
      groups: [
        { key: "tag-todo", blocks: [] },
        { key: "tag-done", blocks: [block("card-1", "Ship it")] },
      ],
    };

    vi.mocked(getBlockView).mockResolvedValue(kanbanRecord);
    vi.mocked(queryBlockViewResults).mockResolvedValueOnce(before).mockResolvedValueOnce(after);
    vi.mocked(updateLoomBlock).mockResolvedValue(block("card-1", "Ship it"));

    render(<BlockCollectionView workspaceId={WS} viewBlockId={VIEW} />);

    await waitFor(() => expect(screen.getByTestId("loom-kanban-view")).toBeTruthy());
    const card = screen.getByTestId("loom-kanban-card");
    const doneLane = screen
      .getAllByTestId("loom-kanban-lane")
      .find((lane) => lane.getAttribute("data-lane-key") === "tag-done")!;

    // Simulate the HTML5 drag from todo -> done.
    const dataTransfer = {
      data: {} as Record<string, string>,
      setData(type: string, value: string) {
        this.data[type] = value;
      },
      getData(type: string) {
        return this.data[type] ?? "";
      },
    };
    fireEvent.dragStart(card, { dataTransfer });
    fireEvent.drop(doneLane, { dataTransfer });

    // REAL mutation: remove old tag, add new tag (each leaves a receipt).
    await waitFor(() =>
      expect(vi.mocked(updateLoomBlock)).toHaveBeenCalledWith(WS, "card-1", {
        add_tags: ["tag-done"],
        remove_tags: ["tag-todo"],
      }),
    );
    // Then re-query (authority, not local state).
    await waitFor(() => expect(vi.mocked(queryBlockViewResults)).toHaveBeenCalledTimes(2));
  });
});

describe("BlockCollectionView calendar", () => {
  it("buckets rows by their real journal_date field", async () => {
    const calRecord: BlockViewRecord = {
      block: block(VIEW, "June"),
      definition: { kind: "calendar", calendar_date_field: "journal_date" },
    };
    const results: BlockViewResults = {
      kind: "calendar",
      blocks: [
        block("j1", "Day 15", { journal_date: "2026-06-15" }),
        block("j2", "Day 20", { journal_date: "2026-06-20" }),
      ],
      total_returned: 2,
    };
    vi.mocked(getBlockView).mockResolvedValue(calRecord);
    vi.mocked(queryBlockViewResults).mockResolvedValue(results);

    render(<BlockCollectionView workspaceId={WS} viewBlockId={VIEW} />);

    await waitFor(() => expect(screen.getByTestId("loom-calendar-view")).toBeTruthy());
    const days = screen.getAllByTestId("loom-calendar-day").map((d) => d.getAttribute("data-date"));
    expect(days).toEqual(["2026-06-15", "2026-06-20"]);
  });
});
