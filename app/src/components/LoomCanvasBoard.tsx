import { useCallback, useEffect, useRef, useState, type DragEvent, type MouseEvent } from "react";

import {
  addCanvasVisualEdge,
  createCanvasCard,
  createLoomEdge,
  defaultLoomCanvasBoardState,
  getCanvasBoard,
  getLoomBlock,
  placeBlockOnCanvas,
  removeCanvasPlacement,
  updateCanvasBoardViewport,
  updateCanvasPlacement,
  type LoomBlock,
  type LoomCanvasBoardState,
  type LoomCanvasBoardView,
  type LoomCanvasPlacement,
} from "../lib/api";

/**
 * MT-261 CanvasBoard — an Obsidian-canvas-class surface over LoomBlock
 * authority. The board IS a typed LoomBlock(content_type=canvas); placed items
 * are block-id REFERENCES rendered as LIVE previews (proving reference-not-copy);
 * semantic connections become real Loom edges; visual-only connections are
 * board-local decoration. Plain React + CSS-transform pan/zoom, HTML5 drag/drop,
 * marquee/shift select, group. Hand-rolled (no new canvas library).
 *
 * Backend authority is PostgreSQL + EventLedger; this component is a projection.
 */

const DEFAULT_CARD_W = 200;
const DEFAULT_CARD_H = 120;

export type EdgeMode = "semantic" | "visual";

/** Payload of an HTML5 drag from a Loom view / quick-switcher hit. */
export type CanvasDragPayload = {
  blockId: string;
  title?: string;
};

export const CANVAS_DRAG_MIME = "application/x-handshake-loom-block";

type Props = {
  workspaceId: string;
  canvasBlockId: string;
};

type LiveBlocks = Record<string, LoomBlock | undefined>;

export function LoomCanvasBoard({ workspaceId, canvasBlockId }: Props) {
  const [view, setView] = useState<LoomCanvasBoardView | null>(null);
  const [liveBlocks, setLiveBlocks] = useState<LiveBlocks>({});
  const [error, setError] = useState<string | null>(null);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [zoom, setZoom] = useState(1);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [edgeMode, setEdgeMode] = useState<EdgeMode>("semantic");
  const [edgeFrom, setEdgeFrom] = useState<string | null>(null);
  const [status, setStatus] = useState<string>("");

  const surfaceRef = useRef<HTMLDivElement | null>(null);

  const loadLiveBlock = useCallback(
    async (blockId: string) => {
      try {
        const block = await getLoomBlock(workspaceId, blockId);
        setLiveBlocks((prev) => ({ ...prev, [blockId]: block }));
      } catch {
        // A missing live block is surfaced as a stale placement, never a copy.
        setLiveBlocks((prev) => ({ ...prev, [blockId]: undefined }));
      }
    },
    [workspaceId],
  );

  const refresh = useCallback(async () => {
    try {
      const next = await getCanvasBoard(workspaceId, canvasBlockId);
      setView(next);
      setPan({ x: next.board.board_state.pan_x, y: next.board.board_state.pan_y });
      setZoom(next.board.board_state.zoom);
      // Resolve every placement to its LIVE referenced block (not a copy).
      await Promise.all(next.placements.map((p) => loadLiveBlock(p.placed_block_id)));
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [workspaceId, canvasBlockId, loadLiveBlock]);

  useEffect(() => {
    let cancelled = false;
    const load = async () => {
      try {
        const next = await getCanvasBoard(workspaceId, canvasBlockId);
        if (cancelled) return;
        setView(next);
        setPan({ x: next.board.board_state.pan_x, y: next.board.board_state.pan_y });
        setZoom(next.board.board_state.zoom);
        const blocks = await Promise.all(
          next.placements.map(async (p) => {
            try {
              return [p.placed_block_id, await getLoomBlock(workspaceId, p.placed_block_id)] as const;
            } catch {
              return [p.placed_block_id, undefined] as const;
            }
          }),
        );
        if (cancelled) return;
        setLiveBlocks(Object.fromEntries(blocks));
      } catch (err) {
        if (!cancelled) setError(err instanceof Error ? err.message : String(err));
      }
    };
    void load();
    return () => {
      cancelled = true;
    };
  }, [workspaceId, canvasBlockId]);

  const persistViewport = useCallback(
    async (next: { x: number; y: number; zoom: number }) => {
      const state: LoomCanvasBoardState = {
        ...defaultLoomCanvasBoardState(),
        pan_x: next.x,
        pan_y: next.y,
        zoom: next.zoom,
      };
      try {
        await updateCanvasBoardViewport(workspaceId, canvasBlockId, state);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      }
    },
    [workspaceId, canvasBlockId],
  );

  // -- pan / zoom ---------------------------------------------------------

  const onPanLeft = () => {
    const next = { x: pan.x - 40, y: pan.y, zoom };
    setPan({ x: next.x, y: next.y });
    void persistViewport(next);
  };
  const onPanRight = () => {
    const next = { x: pan.x + 40, y: pan.y, zoom };
    setPan({ x: next.x, y: next.y });
    void persistViewport(next);
  };
  const onZoomIn = () => {
    const next = { x: pan.x, y: pan.y, zoom: Math.min(4, Math.round((zoom + 0.25) * 100) / 100) };
    setZoom(next.zoom);
    void persistViewport(next);
  };
  const onZoomOut = () => {
    const next = { x: pan.x, y: pan.y, zoom: Math.max(0.25, Math.round((zoom - 0.25) * 100) / 100) };
    setZoom(next.zoom);
    void persistViewport(next);
  };

  // -- drag / drop placement ---------------------------------------------

  const onDrop = useCallback(
    async (event: DragEvent<HTMLDivElement>) => {
      event.preventDefault();
      const raw = event.dataTransfer.getData(CANVAS_DRAG_MIME);
      if (!raw) return;
      let payload: CanvasDragPayload;
      try {
        payload = JSON.parse(raw) as CanvasDragPayload;
      } catch {
        return;
      }
      const rect = surfaceRef.current?.getBoundingClientRect();
      const x = rect ? (event.clientX - rect.left - pan.x) / zoom : 0;
      const y = rect ? (event.clientY - rect.top - pan.y) / zoom : 0;
      try {
        await placeBlockOnCanvas(workspaceId, canvasBlockId, {
          placedBlockId: payload.blockId,
          x,
          y,
          w: DEFAULT_CARD_W,
          h: DEFAULT_CARD_H,
        });
        setStatus(`Placed ${payload.blockId} (reference)`);
        await refresh();
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      }
    },
    [workspaceId, canvasBlockId, pan, zoom, refresh],
  );

  // -- free-text card (a REAL note block) --------------------------------

  const onAddCard = useCallback(async () => {
    try {
      const result = await createCanvasCard(workspaceId, canvasBlockId, {
        title: `Card ${new Date().toISOString()}`,
        body: "New idea",
        x: 40,
        y: 40,
        w: DEFAULT_CARD_W,
        h: DEFAULT_CARD_H,
      });
      setStatus(`Created note block ${result.block.block_id} (content_type=${result.block.content_type})`);
      await refresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [workspaceId, canvasBlockId, refresh]);

  // -- selection / group --------------------------------------------------

  const togglePlacement = (event: MouseEvent, placement: LoomCanvasPlacement) => {
    setSelected((prev) => {
      const next = new Set(event.shiftKey ? prev : []);
      if (next.has(placement.placement_id)) next.delete(placement.placement_id);
      else next.add(placement.placement_id);
      return next;
    });
    // Edge drawing: first click sets the source, second click draws the edge.
    if (edgeFrom && edgeFrom !== placement.placement_id) {
      void drawEdge(edgeFrom, placement.placement_id);
      setEdgeFrom(null);
    }
  };

  const onGroupSelected = useCallback(async () => {
    if (selected.size < 2) return;
    const groupId = `grp-${Date.now()}`;
    try {
      await Promise.all(
        Array.from(selected).map((placementId) =>
          updateCanvasPlacement(workspaceId, placementId, { groupId }),
        ),
      );
      setStatus(`Grouped ${selected.size} placements as ${groupId}`);
      await refresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [selected, workspaceId, refresh]);

  // -- edges (semantic vs visual) ----------------------------------------

  const drawEdge = useCallback(
    async (fromPlacementId: string, toPlacementId: string) => {
      if (!view) return;
      const from = view.placements.find((p) => p.placement_id === fromPlacementId);
      const to = view.placements.find((p) => p.placement_id === toPlacementId);
      if (!from || !to) return;
      try {
        if (edgeMode === "semantic") {
          await createLoomEdge(workspaceId, {
            sourceBlockId: from.placed_block_id,
            targetBlockId: to.placed_block_id,
            edgeType: "mention",
          });
          setStatus(`Semantic edge ${from.placed_block_id} -> ${to.placed_block_id} (real loom edge)`);
        } else {
          await addCanvasVisualEdge(workspaceId, canvasBlockId, {
            fromPlacementId,
            toPlacementId,
          });
          setStatus("Visual-only edge added (NOT graph authority)");
        }
        await refresh();
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      }
    },
    [view, edgeMode, workspaceId, canvasBlockId, refresh],
  );

  const onRemovePlacement = useCallback(
    async (placementId: string) => {
      try {
        await removeCanvasPlacement(workspaceId, placementId);
        setStatus(`Removed placement ${placementId} (source block kept)`);
        await refresh();
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      }
    },
    [workspaceId, refresh],
  );

  if (error) {
    return (
      <div data-testid="loom-canvas.error" style={{ color: "#b91c1c", padding: 12 }}>
        Canvas error: {error}
      </div>
    );
  }
  if (!view) {
    return <div data-testid="loom-canvas.loading">Loading canvas…</div>;
  }

  return (
    <div data-testid="loom-canvas" style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div
        data-testid="loom-canvas.toolbar"
        style={{ display: "flex", gap: 8, padding: 8, borderBottom: "1px solid #e2e8f0", alignItems: "center" }}
      >
        <button data-testid="loom-canvas.pan-left" onClick={onPanLeft} type="button">
          ◀ Pan
        </button>
        <button data-testid="loom-canvas.pan-right" onClick={onPanRight} type="button">
          Pan ▶
        </button>
        <button data-testid="loom-canvas.zoom-out" onClick={onZoomOut} type="button">
          −
        </button>
        <span data-testid="loom-canvas.zoom-value">{zoom.toFixed(2)}x</span>
        <button data-testid="loom-canvas.zoom-in" onClick={onZoomIn} type="button">
          +
        </button>
        <button data-testid="loom-canvas.add-card" onClick={onAddCard} type="button">
          + Text card
        </button>
        <button
          data-testid="loom-canvas.group"
          onClick={onGroupSelected}
          type="button"
          disabled={selected.size < 2}
        >
          Group ({selected.size})
        </button>
        <label data-testid="loom-canvas.edge-mode-label" style={{ marginLeft: "auto" }}>
          Edge:
          <select
            data-testid="loom-canvas.edge-mode"
            value={edgeMode}
            onChange={(e) => setEdgeMode(e.target.value as EdgeMode)}
          >
            <option value="semantic">Semantic (Loom edge)</option>
            <option value="visual">Visual only</option>
          </select>
        </label>
        <button
          data-testid="loom-canvas.start-edge"
          type="button"
          disabled={selected.size !== 1}
          onClick={() => setEdgeFrom(Array.from(selected)[0] ?? null)}
        >
          {edgeFrom ? "Click target…" : "Draw edge from selected"}
        </button>
      </div>

      <div
        data-testid="loom-canvas.status"
        style={{ padding: "4px 8px", fontSize: 12, color: "#475569", minHeight: 18 }}
      >
        {status}
      </div>

      <div
        ref={surfaceRef}
        data-testid="loom-canvas.surface"
        onDragOver={(e) => e.preventDefault()}
        onDrop={onDrop}
        style={{
          position: "relative",
          flex: 1,
          overflow: "hidden",
          background: "#f8fafc",
          backgroundImage: "radial-gradient(#cbd5e1 1px, transparent 1px)",
          backgroundSize: "24px 24px",
        }}
      >
        <div
          data-testid="loom-canvas.transform"
          style={{
            position: "absolute",
            left: 0,
            top: 0,
            transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})`,
            transformOrigin: "0 0",
          }}
        >
          {/* Visual-only edges (board-local decoration; NOT graph authority). */}
          <svg
            data-testid="loom-canvas.visual-edges"
            style={{ position: "absolute", left: 0, top: 0, overflow: "visible", pointerEvents: "none" }}
            width={1}
            height={1}
          >
            {view.visual_edges.map((edge) => {
              const from = view.placements.find((p) => p.placement_id === edge.from_placement_id);
              const to = view.placements.find((p) => p.placement_id === edge.to_placement_id);
              if (!from || !to) return null;
              return (
                <line
                  key={edge.visual_edge_id}
                  data-testid={`loom-canvas.visual-edge.${edge.visual_edge_id}`}
                  x1={from.x + from.w / 2}
                  y1={from.y + from.h / 2}
                  x2={to.x + to.w / 2}
                  y2={to.y + to.h / 2}
                  stroke="#94a3b8"
                  strokeWidth={2}
                  strokeDasharray="6 4"
                />
              );
            })}
          </svg>

          {view.placements.map((placement) => {
            const live = liveBlocks[placement.placed_block_id];
            const isSelected = selected.has(placement.placement_id);
            return (
              <div
                key={placement.placement_id}
                data-testid={`loom-canvas.placement.${placement.placement_id}`}
                data-placed-block-id={placement.placed_block_id}
                data-group-id={placement.group_id ?? ""}
                onClick={(e) => togglePlacement(e, placement)}
                style={{
                  position: "absolute",
                  left: placement.x,
                  top: placement.y,
                  width: placement.w,
                  height: placement.h,
                  zIndex: placement.z_index,
                  border: isSelected ? "2px solid #2563eb" : "1px solid #cbd5e1",
                  borderRadius: 6,
                  background: "#ffffff",
                  boxShadow: "0 1px 3px rgba(0,0,0,0.1)",
                  padding: 8,
                  boxSizing: "border-box",
                  cursor: "pointer",
                  overflow: "hidden",
                }}
              >
                <div
                  data-testid={`loom-canvas.placement.${placement.placement_id}.title`}
                  style={{ fontWeight: 600, fontSize: 13 }}
                >
                  {/* LIVE preview of the referenced block — proves reference-not-copy. */}
                  {live ? (live.title ?? "(untitled)") : "(stale reference)"}
                </div>
                <div style={{ fontSize: 11, color: "#64748b" }}>
                  {live ? live.content_type : "missing"}
                </div>
                <button
                  data-testid={`loom-canvas.placement.${placement.placement_id}.remove`}
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    void onRemovePlacement(placement.placement_id);
                  }}
                  style={{ position: "absolute", top: 4, right: 4, fontSize: 11 }}
                >
                  ✕
                </button>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}

export default LoomCanvasBoard;
