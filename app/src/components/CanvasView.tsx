import { useEffect, useMemo, useState } from "react";
import {
  CanvasEdge,
  CanvasEdgeInput,
  CanvasNode,
  CanvasNodeInput,
  CanvasWithGraph,
  getCanvas,
  updateCanvasGraph,
} from "../lib/api";
import { ExcalidrawCanvas } from "./ExcalidrawCanvas";
import {
  ExcalidrawArrowElement,
  ExcalidrawElement,
  ExcalidrawEllipseElement,
  ExcalidrawGenericElement,
  ExcalidrawImageElement,
  ExcalidrawLinearElement,
  ExcalidrawRectangleElement,
  ExcalidrawTextElement,
} from "@excalidraw/excalidraw/element/types";
import { BinaryFiles } from "@excalidraw/excalidraw/types";

type Props = {
  canvasId: string | null;
};

const isDevEnv = typeof globalThis !== "undefined" && (globalThis as any).process?.env?.NODE_ENV === "development";
// TODO(dev-tools): Phase 0/1 should expose an in-app debug/logs panel so canvas errors are visible without opening the Tauri console (Roadmap §7.6).

export function CanvasView({ canvasId }: Props) {
  const [canvas, setCanvas] = useState<CanvasWithGraph | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [initialElements, setInitialElements] = useState<readonly ExcalidrawElement[] | null>(null);
  const [initialFiles, setInitialFiles] = useState<BinaryFiles | null>(null);
  const [elements, setElements] = useState<readonly ExcalidrawElement[] | null>(null);
  const [files, setFiles] = useState<BinaryFiles | null>(null);
  const [isSaving, setIsSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [lastSavedAt, setLastSavedAt] = useState<string | null>(null);

  useEffect(() => {
    if (!canvasId) {
      setCanvas(null);
      setError(null);
      setInitialElements(null);
      setInitialFiles(null);
      setElements(null);
      setFiles(null);
      return;
    }
    const load = async () => {
      setLoading(true);
      setError(null);
      setSaveError(null);
      setLastSavedAt(null);
      try {
        const data = await getCanvas(canvasId);
        setCanvas(data);
        const mapped = canvasToElements(data);
        setInitialElements(mapped.elements);
        setInitialFiles(mapped.files);
        setElements(mapped.elements);
        setFiles(mapped.files);
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to load canvas");
      } finally {
        setLoading(false);
      }
    };
    void load();
  }, [canvasId]);

  const stats = useMemo(() => {
    return {
      nodes: canvas?.nodes.length ?? 0,
      edges: canvas?.edges.length ?? 0,
    };
  }, [canvas]);

  const handleSave = async () => {
    if (!canvasId || !canvas || !elements) return;
    setIsSaving(true);
    setSaveError(null);
    try {
      const graph = elementsToGraph(elements, files);
      const updated = await updateCanvasGraph(canvasId, graph.nodes, graph.edges);
      setCanvas(updated);
      const mapped = canvasToElements(updated);
      setInitialElements(mapped.elements);
      setInitialFiles(mapped.files);
      setElements(mapped.elements);
      setFiles(mapped.files);
      setLastSavedAt(new Date().toLocaleTimeString());
    } catch (err) {
      setSaveError(err instanceof Error ? err.message : "Failed to save canvas");
    } finally {
      setIsSaving(false);
    }
  };

  if (!canvasId) {
    return (
      <div className="content-card">
        <h2>Canvas</h2>
        <p className="muted">No canvas selected.</p>
      </div>
    );
  }

  if (loading) {
    return (
      <div className="content-card">
        <h2>Canvas</h2>
        <p className="muted">Loading…</p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="content-card">
        <h2>Canvas</h2>
        <p className="muted">Error: {error}</p>
      </div>
    );
  }

  if (!canvas) return null;

  return (
    <div className="content-card">
      <h2>{canvas.title}</h2>
      <p className="muted">Workspace: {canvas.workspace_id}</p>
      <p className="muted">
        Created: {new Date(canvas.created_at).toLocaleString()} – Updated: {new Date(canvas.updated_at).toLocaleString()}
      </p>
      <div className="document-editor__actions" style={{ marginBottom: "12px", alignItems: "center" }}>
        <span className="muted">
          Nodes: {stats.nodes} • Edges: {stats.edges} {lastSavedAt ? `• Saved at ${lastSavedAt}` : ""}
        </span>
        <div style={{ display: "flex", gap: 8 }}>
          <button onClick={handleSave} disabled={isSaving}>
            {isSaving ? "Saving..." : "Save Canvas"}
          </button>
          <button
            onClick={async () => {
              if (!canvasId) return;
              setLoading(true);
              setSaveError(null);
              try {
                const refreshed = await getCanvas(canvasId);
                setCanvas(refreshed);
                const mapped = canvasToElements(refreshed);
                setInitialElements(mapped.elements);
                setInitialFiles(mapped.files);
                setElements(mapped.elements);
                setFiles(mapped.files);
              } catch (err) {
                setError(err instanceof Error ? err.message : "Failed to reload canvas");
              } finally {
                setLoading(false);
              }
            }}
            disabled={isSaving}
          >
            Reload
          </button>
        </div>
      </div>

      <div className="canvas-editor">
        {initialElements && (
          <ExcalidrawCanvas
            initialElements={initialElements}
            initialFiles={initialFiles ?? undefined}
            onChange={(updatedElements, updatedFiles) => {
              setElements(updatedElements);
              setFiles(updatedFiles);
            }}
          />
        )}
        {saveError && <p className="muted">Error: {saveError}</p>}
      </div>
    </div>
  );
}

const DEFAULT_NODE_WIDTH = 240;
const DEFAULT_NODE_HEIGHT = 140;

type ElementSnapshot = {
  id: string;
  type: ExcalidrawElement["type"];
  x: number;
  y: number;
  width: number;
  height: number;
  angle: number;
  strokeColor: string;
  backgroundColor: string;
  fillStyle: ExcalidrawElement["fillStyle"];
  strokeWidth: number;
  strokeStyle: ExcalidrawElement["strokeStyle"];
  roundness: ExcalidrawElement["roundness"];
  roughness: number;
  opacity: number;
  seed: number;
  version: number;
  versionNonce: number;
  groupIds: readonly string[];
  boundElements: ExcalidrawElement["boundElements"];
  link: string | null;
  locked: boolean;
  isDeleted: boolean;
  // Text
  text?: string;
  fontSize?: number;
  fontFamily?: number;
  textAlign?: ExcalidrawTextElement["textAlign"];
  verticalAlign?: ExcalidrawTextElement["verticalAlign"];
  baseline?: number | null;
  containerId?: string | null;
  originalText?: string;
  autoResize?: boolean;
  lineHeight?: number;
  // Linear / arrow
  points?: ExcalidrawLinearElement["points"];
  lastCommittedPoint?: ExcalidrawLinearElement["lastCommittedPoint"];
  startBinding?: ExcalidrawArrowElement["startBinding"];
  endBinding?: ExcalidrawArrowElement["endBinding"];
  startArrowhead?: ExcalidrawArrowElement["startArrowhead"] | null;
  endArrowhead?: ExcalidrawArrowElement["endArrowhead"] | null;
  pressures?: number[] | null;
  simulatePressure?: boolean;
  // Image
  fileId?: string | null;
  status?: ExcalidrawImageElement["status"];
  scale?: ExcalidrawImageElement["scale"];
  crop?: ExcalidrawImageElement["crop"];
  fileData?: { mimeType: string; dataURL: string } | null;
  // Free draw / data passthrough
  data?: unknown;
};

type NodeData = {
  element: ElementSnapshot;
};

function safeNodeData(node: CanvasNode): NodeData {
  if (node && typeof node.data === "object" && node.data !== null) {
    const raw = node.data as Record<string, unknown>;
    const element = raw.element as Partial<ElementSnapshot>;
    if (element && typeof element === "object") {
      return {
        element: withElementDefaults(
          {
            ...(element as ElementSnapshot),
            type: (element as ElementSnapshot).type ?? "text",
          },
          node.id,
        ),
      };
    }
  }
  // Legacy minimal data fallback
  return {
    element: withElementDefaults(
      {
        id: node.id,
        type: "text",
        x: node.position_x,
        y: node.position_y,
        width: DEFAULT_NODE_WIDTH,
        height: DEFAULT_NODE_HEIGHT,
        angle: 0,
        strokeColor: "#0f172a",
        backgroundColor: "#e0f2fe",
        fillStyle: "solid",
        strokeWidth: 1,
        strokeStyle: "solid",
        roughness: 0,
        opacity: 100,
        seed: randomSeed(),
        version: 1,
        versionNonce: randomSeed(),
        roundness: null,
        boundElements: [],
        groupIds: [],
        link: null,
        locked: false,
        isDeleted: false,
        text: "",
        fontSize: 20,
        fontFamily: 1,
        lineHeight: 1.25,
      },
      node.id,
    ),
  };
}

function randomSeed() {
  return Math.floor(Math.random() * 1_000_000_000);
}

function canvasToElements(canvas: CanvasWithGraph): { elements: ExcalidrawElement[]; files: BinaryFiles } {
  const now = Date.now();
  const nodeData = new Map<string, NodeData>();
  const files: BinaryFiles = {};
  const elementsById = new Map<string, ExcalidrawElement>();

  canvas.nodes.forEach((node, index) => {
    const data = safeNodeData(node);
    nodeData.set(node.id, data);
    const el = nodeToElement(node, data, files, now + index);
    if (el) elementsById.set(el.id, el);
  });

  canvas.edges.forEach((edge, index) => {
    const edgeEl = edgeToElement(edge, nodeData, now + 1000 + index);
    if (edgeEl) elementsById.set(edgeEl.id, edgeEl);
  });

  if (isDevEnv) {
    const firstFree = Array.from(elementsById.values()).find((e) => e.type === "freedraw");
    const firstArrow = Array.from(elementsById.values()).find((e) => e.type === "arrow");
    if (firstFree) {
      console.debug("[Canvas load] freedraw reconstructed", {
        id: firstFree.id,
        points: Array.isArray((firstFree as any).points) ? (firstFree as any).points.length : 0,
      });
    }
    if (firstArrow) {
      console.debug("[Canvas load] arrow reconstructed", {
        id: firstArrow.id,
        startArrowhead: (firstArrow as any).startArrowhead,
        endArrowhead: (firstArrow as any).endArrowhead,
        points: Array.isArray((firstArrow as any).points) ? (firstArrow as any).points.length : 0,
      });
    }
  }

  return { elements: Array.from(elementsById.values()), files };
}

function nodeToElement(
  node: CanvasNode,
  data: NodeData,
  files: BinaryFiles,
  timestamp: number,
): ExcalidrawElement | null {
  const snap = withElementDefaults(data.element, node.id);
  const type = snap.type;

  const normalizeLinearPoints = (pts: ExcalidrawLinearElement["points"] | undefined, width: number, height: number) => {
    if (pts && Array.isArray(pts) && pts.length > 0) return pts;
    return [
      [0, 0],
      [width || 0, height || 0],
    ] as ExcalidrawLinearElement["points"];
  };

  const normalizePressures = (pressures: number[] | null | undefined, points: ExcalidrawLinearElement["points"]) => {
    if (!pressures || pressures.length !== points.length) {
      // Safely default to matching length so Excalidraw doesn't crash on mismatch.
      return points.map(() => 0.5);
    }
    return pressures;
  };

  const base: ExcalidrawGenericElement = {
    id: node.id,
    type: type as ExcalidrawGenericElement["type"],
    x: snap.x ?? node.position_x,
    y: snap.y ?? node.position_y,
    strokeColor: snap.strokeColor,
    backgroundColor: snap.backgroundColor,
    fillStyle: snap.fillStyle,
    strokeWidth: snap.strokeWidth,
    strokeStyle: snap.strokeStyle,
    roundness: snap.roundness ?? null,
    roughness: snap.roughness,
    opacity: snap.opacity,
    width: snap.width ?? DEFAULT_NODE_WIDTH,
    height: snap.height ?? DEFAULT_NODE_HEIGHT,
    angle: snap.angle ?? 0,
    seed: snap.seed ?? randomSeed(),
    version: snap.version ?? 1,
    versionNonce: snap.versionNonce ?? randomSeed(),
    index: null,
    isDeleted: snap.isDeleted ?? false,
    groupIds: snap.groupIds ?? [],
    frameId: (snap as any).frameId ?? null,
    boundElements: snap.boundElements ?? [],
    updated: timestamp,
    link: snap.link ?? null,
    locked: snap.locked ?? false,
  };

  if (type === "image") {
    const img: ExcalidrawImageElement = {
      ...(base as ExcalidrawGenericElement),
      type: "image",
      fileId: (snap.fileId as ExcalidrawImageElement["fileId"]) ?? null,
      status: (snap.status as ExcalidrawImageElement["status"]) ?? "saved",
      scale: (snap.scale as [number, number]) ?? [1, 1],
      crop: (snap.crop as ExcalidrawImageElement["crop"]) ?? null,
    };
    if (snap.fileData && snap.fileId) {
      files[snap.fileId] = {
        id: snap.fileId,
        dataURL: snap.fileData.dataURL,
        mimeType: snap.fileData.mimeType,
        created: Date.now(),
        lastRetrieved: Date.now(),
      } as any;
    } else if (snap.fileId && !files[snap.fileId]) {
      // TODO: Image files persistence is not yet implemented; fileId is stored but file data is not persisted.
    }
    return img;
  }

  if (type === "freedraw" || type === "line") {
    const linear: ExcalidrawLinearElement = {
      ...(base as ExcalidrawGenericElement),
    type: type as ExcalidrawLinearElement["type"],
    points: normalizeLinearPoints(snap.points as ExcalidrawLinearElement["points"], snap.width ?? 0, snap.height ?? 0),
    lastCommittedPoint: (snap.lastCommittedPoint as ExcalidrawLinearElement["lastCommittedPoint"]) ?? null,
    startBinding: (snap.startBinding as ExcalidrawArrowElement["startBinding"]) ?? null,
    endBinding: (snap.endBinding as ExcalidrawArrowElement["endBinding"]) ?? null,
    startArrowhead: (snap.startArrowhead as ExcalidrawArrowElement["startArrowhead"]) ?? null,
    endArrowhead: (snap.endArrowhead as ExcalidrawArrowElement["endArrowhead"]) ?? null,
  };
  const pressures = (snap.pressures as number[] | null) ?? null;
  const simulatePressure = snap.simulatePressure ?? false;
  return {
    ...(linear as any),
    pressures: normalizePressures(pressures, linear.points),
    simulatePressure,
  } as ExcalidrawLinearElement;
}

  if (type === "arrow") {
    const startArrowhead =
      snap.startArrowhead !== undefined && snap.startArrowhead !== null ? snap.startArrowhead : null;
    const endArrowhead =
      snap.endArrowhead !== undefined && snap.endArrowhead !== null ? snap.endArrowhead : "arrow";

    const arrow: ExcalidrawArrowElement = {
      ...(base as ExcalidrawGenericElement),
      type: "arrow",
      points: (snap.points as ExcalidrawArrowElement["points"]) ?? [
        [0, 0],
        [snap.width ?? 0, snap.height ?? 0],
      ],
      lastCommittedPoint: (snap.lastCommittedPoint as ExcalidrawArrowElement["lastCommittedPoint"]) ?? null,
      startBinding: (snap.startBinding as ExcalidrawArrowElement["startBinding"]) ?? null,
      endBinding: (snap.endBinding as ExcalidrawArrowElement["endBinding"]) ?? null,
      startArrowhead: startArrowhead as ExcalidrawArrowElement["startArrowhead"],
      endArrowhead: endArrowhead as ExcalidrawArrowElement["endArrowhead"],
      elbowed: false,
    };
    return arrow;
  }

  if (type === "rectangle" || type === "diamond" || type === "ellipse") {
    return { ...(base as ExcalidrawRectangleElement), type } as ExcalidrawRectangleElement | ExcalidrawEllipseElement;
  }

  // Default to text
  const textContent = snap.text ?? "";
  const textEl: ExcalidrawTextElement = {
    ...(base as ExcalidrawGenericElement),
    type: "text",
    fontSize: snap.fontSize ?? 20,
    fontFamily: (snap.fontFamily as ExcalidrawTextElement["fontFamily"]) ?? 1,
    text: textContent,
    textAlign: snap.textAlign ?? "left",
    verticalAlign: snap.verticalAlign ?? "top",
    containerId: snap.containerId ?? null,
    originalText: snap.originalText ?? textContent,
    autoResize: snap.autoResize ?? false,
    lineHeight: (snap.lineHeight as ExcalidrawTextElement["lineHeight"]) ?? 1.25,
  };
  return textEl;
}

function edgeToElement(
  edge: CanvasEdge,
  nodes: Map<string, NodeData>,
  timestamp: number,
): ExcalidrawArrowElement | null {
  const from = nodes.get(edge.from_node_id);
  const to = nodes.get(edge.to_node_id);
  if (!from || !to) return null;

  const snap = withElementDefaults(
    nodes.get(edge.id)?.element ??
      (from.element.type === "arrow" ? from.element : undefined) ??
      (to.element.type === "arrow" ? to.element : undefined),
    edge.id,
  );

  const dx = (to.element.x ?? 0) - (from.element.x ?? 0);
  const dy = (to.element.y ?? 0) - (from.element.y ?? 0);

  const startArrowhead =
    snap.startArrowhead !== undefined && snap.startArrowhead !== null ? snap.startArrowhead : null;
  const endArrowhead = snap.endArrowhead !== undefined && snap.endArrowhead !== null ? snap.endArrowhead : "arrow";

  const arrow: ExcalidrawArrowElement = {
    id: edge.id,
    type: "arrow",
    x: snap.x ?? from.element.x ?? 0,
    y: snap.y ?? from.element.y ?? 0,
    strokeColor: snap.strokeColor,
    backgroundColor: snap.backgroundColor,
    fillStyle: snap.fillStyle,
    strokeWidth: snap.strokeWidth,
    strokeStyle: snap.strokeStyle,
    roundness: snap.roundness ?? null,
    roughness: snap.roughness,
    opacity: snap.opacity,
    width: snap.width ?? (Math.abs(dx) || 1),
    height: snap.height ?? (Math.abs(dy) || 1),
    angle: snap.angle ?? 0,
    seed: snap.seed ?? randomSeed(),
    version: snap.version ?? 1,
    versionNonce: snap.versionNonce ?? randomSeed(),
    index: null,
    isDeleted: snap.isDeleted ?? false,
    groupIds: snap.groupIds ?? [],
    frameId: (snap as any).frameId ?? null,
    boundElements: snap.boundElements ?? [],
    updated: timestamp,
    link: snap.link ?? null,
    locked: snap.locked ?? false,
    points:
      (snap.points as ExcalidrawArrowElement["points"]) ??
      ([
        [0, 0],
        [dx || 1, dy || 1],
      ] as ExcalidrawArrowElement["points"]),
    lastCommittedPoint: (snap.lastCommittedPoint as ExcalidrawArrowElement["lastCommittedPoint"]) ?? null,
    startBinding: {
      ...(snap.startBinding ?? {}),
      elementId: edge.from_node_id,
    } as ExcalidrawArrowElement["startBinding"],
    endBinding: {
      ...(snap.endBinding ?? {}),
      elementId: edge.to_node_id,
    } as ExcalidrawArrowElement["endBinding"],
    startArrowhead: startArrowhead as ExcalidrawArrowElement["startArrowhead"],
    endArrowhead: endArrowhead as ExcalidrawArrowElement["endArrowhead"],
    elbowed: false,
  };

  return arrow;
}

function elementsToGraph(elements: readonly ExcalidrawElement[], files: BinaryFiles | null): {
  nodes: CanvasNodeInput[];
  edges: CanvasEdgeInput[];
} {
  const nodes: CanvasNodeInput[] = [];
  const edges: CanvasEdgeInput[] = [];
  const nodeIds = new Set<string>();
  let loggedFreedraw = false;
  let loggedArrow = false;

  elements.forEach((el) => {
    if (el.isDeleted || el.type === "selection") return;
    const nodeInput: CanvasNodeInput = {
      id: el.id,
      kind: el.type,
      position_x: el.x,
      position_y: el.y,
      data: {
        element: sanitizeElement(el, files ?? {}),
      },
    };
    nodes.push(nodeInput);
    nodeIds.add(el.id);

    if (isDevEnv && !loggedFreedraw && el.type === "freedraw") {
      console.debug("[Canvas save] freedraw snapshot", {
        id: el.id,
        type: el.type,
        points: Array.isArray((el as any).points) ? (el as any).points.length : 0,
      });
      loggedFreedraw = true;
    }
  });

  elements.forEach((el) => {
    if (el.isDeleted || el.type === "selection") return;
    if (isArrowElement(el)) {
      const fromId = el.startBinding?.elementId;
      const toId = el.endBinding?.elementId;
      if (!fromId || !toId) return;
      if (!nodeIds.has(fromId) || !nodeIds.has(toId)) return;
      edges.push({
        id: el.id,
        from_node_id: fromId,
        to_node_id: toId,
        kind: el.type,
      });

      if (isDevEnv && !loggedArrow) {
        console.debug("[Canvas save] arrow snapshot", {
          id: el.id,
          startArrowhead: (el as any).startArrowhead,
          endArrowhead: (el as any).endArrowhead,
          points: Array.isArray((el as any).points) ? (el as any).points.length : 0,
        });
        loggedArrow = true;
      }
    }
  });

  return { nodes, edges };
}

function isArrowElement(el: ExcalidrawElement): el is ExcalidrawArrowElement {
  return el.type === "arrow";
}

function sanitizeElement(el: ExcalidrawElement, files: BinaryFiles): ElementSnapshot {
  const snap: ElementSnapshot = {
    id: el.id,
    type: el.type,
    x: el.x,
    y: el.y,
    width: el.width,
    height: el.height,
    angle: el.angle,
    strokeColor: el.strokeColor,
    backgroundColor: el.backgroundColor,
    fillStyle: el.fillStyle,
    strokeWidth: el.strokeWidth,
    strokeStyle: el.strokeStyle,
    roundness: el.roundness ?? null,
    roughness: el.roughness,
    opacity: el.opacity,
    seed: el.seed,
    version: el.version,
    versionNonce: el.versionNonce,
    boundElements: el.boundElements ?? [],
    link: el.link ?? null,
    locked: el.locked ?? false,
    groupIds: el.groupIds ?? [],
    isDeleted: el.isDeleted ?? false,
    text: (el as any).text,
    fontSize: (el as any).fontSize,
    fontFamily: (el as any).fontFamily,
    textAlign: (el as any).textAlign,
    verticalAlign: (el as any).verticalAlign,
    baseline: (el as any).baseline,
    containerId: (el as any).containerId,
    originalText: (el as any).originalText,
    autoResize: (el as any).autoResize,
    lineHeight: (el as any).lineHeight,
    points: (el as any).points,
    lastCommittedPoint: (el as any).lastCommittedPoint,
    startBinding: (el as any).startBinding,
    endBinding: (el as any).endBinding,
    startArrowhead: (el as any).startArrowhead ?? null,
    endArrowhead: (el as any).endArrowhead ?? null,
    pressures: (el as any).pressures ?? null,
    simulatePressure: (el as any).simulatePressure,
    fileId: (el as any).fileId,
    status: (el as any).status,
    scale: (el as any).scale,
    crop: (el as any).crop,
    data: (el as any).data,
  };

  if (el.type === "image") {
    const fileId = (el as any).fileId as string | null;
    if (fileId && files[fileId]) {
      snap.fileData = {
        mimeType: files[fileId].mimeType,
        dataURL: files[fileId].dataURL,
      };
    } else {
      snap.fileData = null;
    }
  }

  return snap;
}

function withElementDefaults(snap: ElementSnapshot | undefined, fallbackId?: string): ElementSnapshot {
  const widthVal = snap?.width ?? DEFAULT_NODE_WIDTH;
  const heightVal = snap?.height ?? DEFAULT_NODE_HEIGHT;
  const normalizedPoints =
    snap?.points && Array.isArray(snap.points) && snap.points.length > 0
      ? snap.points
      : [
          [0, 0],
          [widthVal || 0, heightVal || 0],
        ];

  return {
    id: snap?.id ?? fallbackId ?? `${randomSeed()}`,
    type: snap?.type ?? "text",
    x: snap?.x ?? 0,
    y: snap?.y ?? 0,
    width: widthVal,
    height: heightVal,
    angle: snap?.angle ?? 0,
    strokeColor: snap?.strokeColor ?? "#0f172a",
    backgroundColor: snap?.backgroundColor ?? "#e0f2fe",
    fillStyle: snap?.fillStyle ?? "solid",
    strokeWidth: snap?.strokeWidth ?? 1,
    strokeStyle: snap?.strokeStyle ?? "solid",
    roughness: snap?.roughness ?? 0,
    opacity: snap?.opacity ?? 100,
    seed: snap?.seed ?? randomSeed(),
    version: snap?.version ?? 1,
    versionNonce: snap?.versionNonce ?? randomSeed(),
    roundness: snap?.roundness ?? null,
    boundElements: snap?.boundElements ?? [],
    link: snap?.link ?? null,
    locked: snap?.locked ?? false,
    groupIds: snap?.groupIds ?? [],
    isDeleted: snap?.isDeleted ?? false,
    text: snap?.text,
    fontSize: snap?.fontSize,
    fontFamily: snap?.fontFamily,
    textAlign: snap?.textAlign,
    verticalAlign: snap?.verticalAlign,
    baseline: snap?.baseline ?? null,
    containerId: snap?.containerId,
    originalText: snap?.originalText,
    autoResize: snap?.autoResize,
    lineHeight: snap?.lineHeight,
    points: normalizedPoints,
    lastCommittedPoint: snap?.lastCommittedPoint,
    startBinding: snap?.startBinding,
    endBinding: snap?.endBinding,
    startArrowhead: snap?.startArrowhead,
    endArrowhead: snap?.endArrowhead,
    pressures: snap?.pressures ?? null,
    simulatePressure: snap?.simulatePressure ?? false,
    fileId: snap?.fileId,
    status: snap?.status,
    scale: snap?.scale,
    crop: snap?.crop,
    data: snap?.data,
    fileData: snap?.fileData ?? null,
  };
}

function runFreedrawSelfTest() {
  if (!isDevEnv) return;
  try {
    const dummySnap: ElementSnapshot = {
      id: "dev-freedraw-test",
      type: "freedraw",
      x: 0,
      y: 0,
      width: 10,
      height: 5,
      angle: 0,
      strokeColor: "#000000",
      backgroundColor: "#ffffff",
      fillStyle: "solid",
      strokeWidth: 1,
      strokeStyle: "solid",
      roundness: null,
      roughness: 0,
      opacity: 100,
      seed: 1,
      version: 1,
      versionNonce: 1,
      groupIds: [],
      boundElements: [],
      link: null,
      locked: false,
      isDeleted: false,
      points: null as any, // simulate legacy/broken data
      pressures: null,
      simulatePressure: false,
    };
    const nd: NodeData = { element: dummySnap };
    const el = nodeToElement(
      {
        id: "dev-freedraw-test",
        kind: "freedraw",
        position_x: 0,
        position_y: 0,
        data: { element: dummySnap },
      } as unknown as CanvasNode,
      nd,
      {},
      Date.now(),
    );
    if (!el || !(el as any).points || (el as any).points.length < 2) {
      console.debug("[Canvas self-test] Freedraw normalization failed", el);
    }
  } catch (err) {
    console.debug("[Canvas self-test] Freedraw normalization threw", err);
  }
}

runFreedrawSelfTest();
