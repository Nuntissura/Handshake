// WP-KERNEL-009 / MT-264 — LoomSearchV2Panel tests.
//
// Mounts the panel in jsdom with injected search/create-view fns and proves:
//   - a query runs the hybrid search and renders ranked results with the
//     ts_headline <mark> highlight rendered as React <mark> nodes,
//   - content_type facets render and clicking one re-runs the scoped search,
//   - clicking a result opens the REAL block in place (reference, not copy),
//   - "Save as view" reuses the MT-262 block-collection view path,
//   - the no-semantic (keyword/fuzzy only) status renders honestly when the
//     backend reports semantic_available=false (no fabricated semantic claim).

import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import { LoomSearchV2Panel } from "./LoomSearchV2Panel";
import type { LoomBlock, LoomSearchV2Response } from "../lib/api";

const WS = "ws-loom-search-v2";

function block(over: Partial<LoomBlock> = {}): LoomBlock {
  return {
    block_id: "blk-1",
    workspace_id: WS,
    content_type: "note",
    document_id: null,
    asset_id: null,
    title: "Migration runbook",
    original_filename: null,
    content_hash: null,
    pinned: false,
    favorite: false,
    journal_date: null,
    created_at: "2026-06-17T00:00:00Z",
    updated_at: "2026-06-17T00:00:00Z",
    imported_at: null,
    derived: {},
    ...over,
  } as LoomBlock;
}

function response(over: Partial<LoomSearchV2Response> = {}): LoomSearchV2Response {
  return {
    hits: [
      {
        block: block(),
        score: 1.234,
        fts_rank: 0.5,
        trgm_sim: 0.3,
        vector_sim: 0.8,
        edge_degree: 2,
        highlight: "the <mark>migration</mark> runbook",
      },
    ],
    content_type_facets: { note: 1, file: 0 },
    semantic_available: true,
    total: 1,
    ...over,
  };
}

describe("LoomSearchV2Panel", () => {
  it("runs the hybrid search and renders ranked, highlighted results", async () => {
    const searchFn = vi.fn().mockResolvedValue(response());
    const onOpenBlock = vi.fn();
    render(<LoomSearchV2Panel workspaceId={WS} onOpenBlock={onOpenBlock} searchFn={searchFn} />);

    fireEvent.change(screen.getByTestId("loom-search-v2.query"), {
      target: { value: "migration" },
    });
    fireEvent.click(screen.getByTestId("loom-search-v2.search"));

    await waitFor(() => expect(searchFn).toHaveBeenCalledWith(WS, expect.objectContaining({ query: "migration" })));
    // The ts_headline <mark> is rendered as a real <mark> node.
    expect(await screen.findByTestId("loom-search-v2.highlight-mark")).toHaveTextContent("migration");
    expect(screen.getByTestId("loom-search-v2.status")).toHaveTextContent("semantic on");
  });

  it("opens the real block in place when a result is clicked (reference, not copy)", async () => {
    const searchFn = vi.fn().mockResolvedValue(response());
    const onOpenBlock = vi.fn();
    render(<LoomSearchV2Panel workspaceId={WS} onOpenBlock={onOpenBlock} searchFn={searchFn} />);

    fireEvent.change(screen.getByTestId("loom-search-v2.query"), { target: { value: "migration" } });
    fireEvent.click(screen.getByTestId("loom-search-v2.search"));
    const result = await screen.findByTestId("loom-search-v2.result.blk-1");
    fireEvent.click(result);
    expect(onOpenBlock).toHaveBeenCalledWith("blk-1");
  });

  it("applies a content_type facet by re-running the scoped search", async () => {
    const searchFn = vi.fn().mockResolvedValue(response());
    render(<LoomSearchV2Panel workspaceId={WS} onOpenBlock={vi.fn()} searchFn={searchFn} />);

    fireEvent.change(screen.getByTestId("loom-search-v2.query"), { target: { value: "migration" } });
    fireEvent.click(screen.getByTestId("loom-search-v2.search"));
    await screen.findByTestId("loom-search-v2.facet.note");
    fireEvent.click(screen.getByTestId("loom-search-v2.facet.note"));
    await waitFor(() =>
      expect(searchFn).toHaveBeenLastCalledWith(WS, expect.objectContaining({ contentType: "note" })),
    );
  });

  it("saves the search as a Loom view via the MT-262 view path", async () => {
    const searchFn = vi.fn().mockResolvedValue(response());
    const createViewFn = vi
      .fn()
      .mockResolvedValue({ block: block({ block_id: "view-1", content_type: "view_def" }), definition: { kind: "table" } });
    render(
      <LoomSearchV2Panel workspaceId={WS} onOpenBlock={vi.fn()} searchFn={searchFn} createViewFn={createViewFn} />,
    );
    fireEvent.change(screen.getByTestId("loom-search-v2.query"), { target: { value: "migration" } });
    fireEvent.click(screen.getByTestId("loom-search-v2.search"));
    await screen.findByTestId("loom-search-v2.result.blk-1");
    fireEvent.click(screen.getByTestId("loom-search-v2.save-view"));
    await waitFor(() => expect(createViewFn).toHaveBeenCalled());
    expect(screen.getByTestId("loom-search-v2.view-status")).toHaveTextContent("view-1");
  });

  it("honestly reports keyword/fuzzy-only when no semantic model is configured", async () => {
    const searchFn = vi.fn().mockResolvedValue(response({ semantic_available: false }));
    render(<LoomSearchV2Panel workspaceId={WS} onOpenBlock={vi.fn()} searchFn={searchFn} />);
    fireEvent.change(screen.getByTestId("loom-search-v2.query"), { target: { value: "migration" } });
    fireEvent.click(screen.getByTestId("loom-search-v2.search"));
    await waitFor(() =>
      expect(screen.getByTestId("loom-search-v2.status")).toHaveTextContent("keyword/fuzzy only"),
    );
  });
});
