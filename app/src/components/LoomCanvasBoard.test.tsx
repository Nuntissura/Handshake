// WP-KERNEL-009 / MT-261 — LoomCanvasBoard tests.
//
// Mounts the canvas in jsdom with the api module mocked and proves:
//   - placements render a LIVE preview of the referenced block (reference,
//     not a copy),
//   - removing a placement calls removeCanvasPlacement (source block untouched),
//   - free-text card creation calls createCanvasCard (a real note block),
//   - the edge-mode toggle routes a connection to createLoomEdge (semantic) vs
//     addCanvasVisualEdge (visual-only) — the two are never conflated,
//   - pan/zoom controls persist through updateCanvasBoardViewport.

import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { beforeEach, describe, it, expect, vi } from "vitest";
import { LoomCanvasBoard } from "./LoomCanvasBoard";
import {
  addCanvasVisualEdge,
  createCanvasCard,
  createLoomEdge,
  getCanvasBoard,
  getLoomBlock,
  removeCanvasPlacement,
  updateCanvasBoardViewport,
  type LoomCanvasBoardView,
} from "../lib/api";

vi.mock("../lib/api", async () => {
  const actual = await vi.importActual<typeof import("../lib/api")>("../lib/api");
  return {
    ...actual,
    getCanvasBoard: vi.fn(),
    getLoomBlock: vi.fn(),
    placeBlockOnCanvas: vi.fn(),
    createCanvasCard: vi.fn(),
    updateCanvasPlacement: vi.fn(),
    removeCanvasPlacement: vi.fn(),
    addCanvasVisualEdge: vi.fn(),
    createLoomEdge: vi.fn(),
    updateCanvasBoardViewport: vi.fn(),
  };
});

const WS = "ws-canvas-test";
const CANVAS = "canvas-blk-1";

function boardView(): LoomCanvasBoardView {
  return {
    board: {
      block_id: CANVAS,
      workspace_id: WS,
      board_state: { schema_id: "hsk.loom_canvas_board@1", pan_x: 0, pan_y: 0, zoom: 1 },
      created_at: "2026-06-17T00:00:00Z",
      updated_at: "2026-06-17T00:00:00Z",
      event_ledger_event_id: "EV-1",
    },
    placements: [
      {
        placement_id: "LCP-aaa",
        canvas_block_id: CANVAS,
        workspace_id: WS,
        placed_block_id: "blk-a",
        x: 10,
        y: 10,
        w: 200,
        h: 120,
        z_index: 0,
        group_id: null,
        created_at: "2026-06-17T00:00:00Z",
        updated_at: "2026-06-17T00:00:00Z",
      },
      {
        placement_id: "LCP-bbb",
        canvas_block_id: CANVAS,
        workspace_id: WS,
        placed_block_id: "blk-b",
        x: 300,
        y: 10,
        w: 200,
        h: 120,
        z_index: 1,
        group_id: null,
        created_at: "2026-06-17T00:00:00Z",
        updated_at: "2026-06-17T00:00:00Z",
      },
    ],
    visual_edges: [],
  };
}

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(getCanvasBoard).mockResolvedValue(boardView());
  vi.mocked(getLoomBlock).mockImplementation(async (_ws, blockId) =>
    blockId === "blk-a"
      ? ({ block_id: "blk-a", title: "Alpha note", content_type: "note" } as never)
      : ({ block_id: "blk-b", title: "Beta note", content_type: "note" } as never),
  );
  vi.mocked(createCanvasCard).mockResolvedValue({
    block: { block_id: "blk-card", title: "Card", content_type: "note" } as never,
    rich_document_id: "KRD-1",
    placement: boardView().placements[0],
  });
  vi.mocked(removeCanvasPlacement).mockResolvedValue(undefined);
  vi.mocked(createLoomEdge).mockResolvedValue({} as never);
  vi.mocked(addCanvasVisualEdge).mockResolvedValue({} as never);
  vi.mocked(updateCanvasBoardViewport).mockResolvedValue(boardView().board);
});

describe("LoomCanvasBoard", () => {
  it("renders live previews of referenced blocks (reference, not copy)", async () => {
    render(<LoomCanvasBoard workspaceId={WS} canvasBlockId={CANVAS} />);
    await waitFor(() => expect(screen.getByTestId("loom-canvas")).toBeInTheDocument());
    expect(screen.getByTestId("loom-canvas.placement.LCP-aaa.title")).toHaveTextContent("Alpha note");
    expect(screen.getByTestId("loom-canvas.placement.LCP-bbb.title")).toHaveTextContent("Beta note");
    // The live block was resolved via getLoomBlock per placement (live ref).
    expect(getLoomBlock).toHaveBeenCalledWith(WS, "blk-a");
    expect(getLoomBlock).toHaveBeenCalledWith(WS, "blk-b");
  });

  it("removes a placement without touching the source block", async () => {
    render(<LoomCanvasBoard workspaceId={WS} canvasBlockId={CANVAS} />);
    await waitFor(() => expect(screen.getByTestId("loom-canvas.placement.LCP-aaa")).toBeInTheDocument());
    fireEvent.click(screen.getByTestId("loom-canvas.placement.LCP-aaa.remove"));
    await waitFor(() => expect(removeCanvasPlacement).toHaveBeenCalledWith(WS, "LCP-aaa"));
  });

  it("creates a free-text card as a real note block", async () => {
    render(<LoomCanvasBoard workspaceId={WS} canvasBlockId={CANVAS} />);
    await waitFor(() => expect(screen.getByTestId("loom-canvas.add-card")).toBeInTheDocument());
    fireEvent.click(screen.getByTestId("loom-canvas.add-card"));
    await waitFor(() => expect(createCanvasCard).toHaveBeenCalledTimes(1));
    await waitFor(() =>
      expect(screen.getByTestId("loom-canvas.status")).toHaveTextContent("content_type=note"),
    );
  });

  it("routes a SEMANTIC connection to a real Loom edge", async () => {
    render(<LoomCanvasBoard workspaceId={WS} canvasBlockId={CANVAS} />);
    await waitFor(() => expect(screen.getByTestId("loom-canvas.placement.LCP-aaa")).toBeInTheDocument());
    // edge mode defaults to semantic. Select source, start edge, click target.
    fireEvent.click(screen.getByTestId("loom-canvas.placement.LCP-aaa"));
    fireEvent.click(screen.getByTestId("loom-canvas.start-edge"));
    fireEvent.click(screen.getByTestId("loom-canvas.placement.LCP-bbb"));
    await waitFor(() =>
      expect(createLoomEdge).toHaveBeenCalledWith(WS, {
        sourceBlockId: "blk-a",
        targetBlockId: "blk-b",
        edgeType: "mention",
      }),
    );
    expect(addCanvasVisualEdge).not.toHaveBeenCalled();
  });

  it("routes a VISUAL-ONLY connection to a board-local edge (never a Loom edge)", async () => {
    render(<LoomCanvasBoard workspaceId={WS} canvasBlockId={CANVAS} />);
    await waitFor(() => expect(screen.getByTestId("loom-canvas.placement.LCP-aaa")).toBeInTheDocument());
    fireEvent.change(screen.getByTestId("loom-canvas.edge-mode"), { target: { value: "visual" } });
    fireEvent.click(screen.getByTestId("loom-canvas.placement.LCP-aaa"));
    fireEvent.click(screen.getByTestId("loom-canvas.start-edge"));
    fireEvent.click(screen.getByTestId("loom-canvas.placement.LCP-bbb"));
    await waitFor(() =>
      expect(addCanvasVisualEdge).toHaveBeenCalledWith(WS, CANVAS, {
        fromPlacementId: "LCP-aaa",
        toPlacementId: "LCP-bbb",
      }),
    );
    expect(createLoomEdge).not.toHaveBeenCalled();
  });

  it("persists pan/zoom through the viewport endpoint", async () => {
    render(<LoomCanvasBoard workspaceId={WS} canvasBlockId={CANVAS} />);
    await waitFor(() => expect(screen.getByTestId("loom-canvas.zoom-in")).toBeInTheDocument());
    fireEvent.click(screen.getByTestId("loom-canvas.zoom-in"));
    await waitFor(() => expect(updateCanvasBoardViewport).toHaveBeenCalledTimes(1));
    expect(screen.getByTestId("loom-canvas.zoom-value")).toHaveTextContent("1.25x");
  });
});
