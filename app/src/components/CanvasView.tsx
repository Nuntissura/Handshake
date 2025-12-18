import { useEffect, useMemo, useState } from "react";
import {
  CanvasEdge,
  CanvasEdgeInput,
  CanvasNode,
  CanvasNodeInput,
  CanvasWithGraph,
  getCanvas,
  deleteCanvas,
  updateCanvasGraph,
} from "../lib/api";
import { ExcalidrawCanvas } from "./ExcalidrawCanvas";
import { CanvasHeader } from "./CanvasHeader";
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
import { BinaryFileData, BinaryFiles } from "@excalidraw/excalidraw/types";
import { logEvent } from "../state/debugEvents";

type Props = {
  canvasId: string | null;
  onDeleted: () => void;
};

const isDevEnv =
  typeof globalThis !== "undefined" &&
  (globalThis as { process?: { env?: Record<string, unknown> } }).process?.env?.NODE_ENV === "development";

export function CanvasView({ canvasId, onDeleted }: Props) {
  const [canvas, setCanvas] = useState<CanvasWithGraph | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [initialElements, setInitialElements] = useState<readonly ExcalidrawElement[] | null>(null);
  const [initialFiles, setInitialFiles] = useState<BinaryFiles | null>(null);
  const [elements, setElements] = useState<readonly ExcalidrawElement[] | null>(null);
  const [files, setFiles] = useState<BinaryFiles | null>(null);
  const [isSaving, setIsSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [isDeleting, setIsDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);
  const [lastSavedAt, setLastSavedAt] = useState<string | null>(null);

  useEffect(() => {
    if (!canvasId) {
      setCanvas(null);
      setError(null);
      setInitialElements(null);
      setInitialFiles(null);
      setElements(null);
      setFiles(null);
      setDeleteError(null);
      setIsDeleting(false);
      return;
    }
    const load = async () => {
      setLoading(true);
      setError(null);
      setSaveError(null);
      setDeleteError(null);
      setLastSavedAt(null);
      try {
        const data = await getCanvas(canvasId);
        setCanvas(data);
        const mapped = canvasToElements(data);
        setInitialElements(mapped.elements);
        setInitialFiles(mapped.files);
        setElements(mapped.elements);
        setFiles(mapped.files);
        logEvent({ type: "canvas-load", targetId: canvasId, result: "ok" });
      } catch (err) {
        if (isNotFound(err)) {
          window.dispatchEvent(new CustomEvent("handshake:canvas-deleted", { detail: { canvasId, workspaceId: null } }));
          onDeleted();
          return;
        }
        setError(err instanceof Error ? err.message : "Failed to load canvas");
        logEvent({ type: "canvas-load", targetId: canvasId, result: "error", message: String(err) });
      } finally {
        setLoading(false);
      }
    };
    void load();
  }, [canvasId, onDeleted]);

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
    setDeleteError(null);
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
      logEvent({ type: "canvas-save", targetId: canvasId, result: "ok" });
    } catch (err) {
      setSaveError(err instanceof Error ? err.message : "Failed to save canvas");
      logEvent({ type: "canvas-save", targetId: canvasId, result: "error", message: String(err) });
    } finally {
      setIsSaving(false);
    }
  };

  const handleReload = async () => {
    if (!canvasId) return;
    setLoading(true);
    setSaveError(null);
    setDeleteError(null);
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
  };

  const handleDelete = async () => {
    if (!canvasId) return;
    const confirmed = window.confirm("Delete this canvas? This cannot be undone.");
    if (!confirmed) return;
    setIsDeleting(true);
    setDeleteError(null);
    try {
      await deleteCanvas(canvasId);
      const workspaceId = canvas?.workspace_id ?? null;
      window.dispatchEvent(new CustomEvent("handshake:canvas-deleted", { detail: { canvasId, workspaceId } }));
      logEvent({ type: "canvas-delete", targetId: canvasId, result: "ok" });
      onDeleted();
    } catch (err) {
      if (isNotFound(err)) {
        const workspaceId = canvas?.workspace_id ?? null;
        window.dispatchEvent(new CustomEvent("handshake:canvas-deleted", { detail: { canvasId, workspaceId } }));
        logEvent({ type: "canvas-delete", targetId: canvasId, result: "ok" });
        onDeleted();
        return;
      }
      const message = err instanceof Error ? err.message : "Failed to delete canvas";
      setDeleteError(message);
      logEvent({ type: "canvas-delete", targetId: canvasId, result: "error", message: String(err) });
    } finally {
      setIsDeleting(false);
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
        <p className="muted">Loading...</p>
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
      <CanvasHeader
        title={canvas.title}
        workspaceId={canvas.workspace_id}
        createdAt={canvas.created_at}
        updatedAt={canvas.updated_at}
        stats={stats}
        lastSavedAt={lastSavedAt}
        isSaving={isSaving}
        isDeleting={isDeleting}
        onSave={handleSave}
        onReload={handleReload}
        onDelete={handleDelete}
      />

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
        {deleteError && <p className="muted">Error: {deleteError}</p>}
      </div>
    </div>
  );
}

const DEFAULT_NODE_WIDTH = 240;
const DEFAULT_NODE_HEIGHT = 140;

// Helper types and functions remain unchanged from the previous version.

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
  points?: ExcalidrawLinearElement["points"];
  lastCommittedPoint?: ExcalidrawLinearElement["lastCommittedPoint"];
  startBinding?: ExcalidrawArrowElement["startBinding"];
  endBinding?: ExcalidrawArrowElement["endBinding"];
  startArrowhead?: ExcalidrawArrowElement["startArrowhead"] | null;
  endArrowhead?: ExcalidrawArrowElement["endArrowhead"] | null;
  pressures?: number[] | null;
  simulatePressure?: boolean;
  fileId?: string | null;
  status?: ExcalidrawImageElement["status"];
  scale?: ExcalidrawImageElement["scale"];
  crop?: ExcalidrawImageElement["crop"];
  fileData?: { mimeType: string; dataURL: string } | null;
  frameId?: string | null;
  data?: unknown;
};

type NodeData = {
  element: ElementSnapshot;
};

type TextLikeElement = ExcalidrawElement & {
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
};

type LinearLikeElement = ExcalidrawElement & {
  points?: ExcalidrawLinearElement["points"];
  lastCommittedPoint?: ExcalidrawLinearElement["lastCommittedPoint"];
  startBinding?: ExcalidrawArrowElement["startBinding"];
  endBinding?: ExcalidrawArrowElement["endBinding"];
  startArrowhead?: ExcalidrawArrowElement["startArrowhead"] | null;
  endArrowhead?: ExcalidrawArrowElement["endArrowhead"] | null;
  pressures?: number[] | null;
  simulatePressure?: boolean;
};

type ImageLikeElement = ExcalidrawElement & {
  fileId?: string | null;
  status?: ExcalidrawImageElement["status"];
  scale?: ExcalidrawImageElement["scale"];
  crop?: ExcalidrawImageElement["crop"];
};

type ElementWithData = ExcalidrawElement & { data?: unknown };

type ElementWithFrame = ExcalidrawElement & { frameId?: string | null };

type FileId = string;

function isTextLike(el: ExcalidrawElement): el is TextLikeElement {
  return "text" in el;
}

function isLinearLike(el: ExcalidrawElement): el is LinearLikeElement {
  return "points" in el;
}

function isImageLike(el: ExcalidrawElement): el is ImageLikeElement {
  return "fileId" in el;
}

function hasDataField(el: ExcalidrawElement): el is ElementWithData {
  return "data" in el;
}

function hasFrameId(el: ExcalidrawElement | ElementSnapshot): el is ElementWithFrame {
  return "frameId" in el;
}

function getBinaryFileData(files: BinaryFiles | null | undefined, fileId: FileId | null | undefined) {
  if (!files || !fileId) return null;
  const file = files[fileId];
  if (
    file &&
    typeof file === "object" &&
    "dataURL" in file &&
    "mimeType" in file &&
    "created" in file &&
    "lastRetrieved" in file
  ) {
    return file as BinaryFileData;
  }
  return null;
}

function logLinearDebug(label: string, el: LinearLikeElement) {
  if (!isDevEnv) return;
  const pointsCount = Array.isArray(el.points) ? el.points.length : 0;
  console.debug(label, {
    id: el.id,
    startArrowhead: "startArrowhead" in el ? el.startArrowhead : undefined,
    endArrowhead: "endArrowhead" in el ? el.endArrowhead : undefined,
    points: pointsCount,
  });
}

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
        roundness: null,
        roughness: 0,
        opacity: 100,
        seed: randomSeed(),
        version: 1,
        versionNonce: randomSeed(),
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

export function canvasToElements(canvas: CanvasWithGraph): { elements: ExcalidrawElement[]; files: BinaryFiles } {
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
    if (firstFree && isLinearLike(firstFree)) {
      logLinearDebug("[Canvas load] freedraw reconstructed", firstFree);
    }
    if (firstArrow && isLinearLike(firstArrow)) {
      logLinearDebug("[Canvas load] arrow reconstructed", firstArrow);
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
    frameId: hasFrameId(snap) ? snap.frameId ?? null : null,
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
      const created = Date.now();
      const fileEntry: BinaryFileData = {
        id: snap.fileId,
        dataURL: snap.fileData.dataURL,
        mimeType: snap.fileData.mimeType,
        created,
        lastRetrieved: created,
      };
      files[snap.fileId] = fileEntry;
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
      ...linear,
      pressures: normalizePressures(pressures, linear.points),
      simulatePressure,
    };
  }

  if (type === "arrow") {
    const startArrowhead =
      snap.startArrowhead !== undefined && snap.startArrowhead !== null ? snap.startArrowhead : null;
    const endArrowhead = snap.endArrowhead !== undefined && snap.endArrowhead !== null ? snap.endArrowhead : "arrow";

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
    width: (snap.width ?? Math.abs(dx)) || 1,
    height: (snap.height ?? Math.abs(dy)) || 1,
    angle: snap.angle ?? 0,
    seed: snap.seed ?? randomSeed(),
    version: snap.version ?? 1,
    versionNonce: snap.versionNonce ?? randomSeed(),
    index: null,
    isDeleted: snap.isDeleted ?? false,
    groupIds: snap.groupIds ?? [],
    frameId: hasFrameId(snap) ? snap.frameId ?? null : null,
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

export function elementsToGraph(elements: readonly ExcalidrawElement[], files: BinaryFiles | null): {
  nodes: CanvasNodeInput[];
  edges: CanvasEdgeInput[];
} {
  const nodes: CanvasNodeInput[] = [];
  const edges: CanvasEdgeInput[] = [];
  const nodeIds = new Set<string>();
  let loggedFreedraw = false;
  let loggedArrow = false;
  const fileMap = files ?? {};

  elements.forEach((el) => {
    if (el.isDeleted || el.type === "selection") return;
    const nodeInput: CanvasNodeInput = {
      id: el.id,
      kind: el.type,
      position_x: el.x,
      position_y: el.y,
      data: {
        element: sanitizeElement(el, fileMap),
      },
    };
    nodes.push(nodeInput);
    nodeIds.add(el.id);

    if (isDevEnv && !loggedFreedraw && el.type === "freedraw" && isLinearLike(el)) {
      logLinearDebug("[Canvas save] freedraw snapshot", el);
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

      if (isDevEnv && !loggedArrow && isLinearLike(el)) {
        logLinearDebug("[Canvas save] arrow snapshot", el);
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
  };

  if (isTextLike(el)) {
    snap.text = el.text;
    snap.fontSize = el.fontSize;
    snap.fontFamily = el.fontFamily;
    snap.textAlign = el.textAlign;
    snap.verticalAlign = el.verticalAlign;
    snap.baseline = el.baseline ?? null;
    snap.containerId = el.containerId;
    snap.originalText = el.originalText;
    snap.autoResize = el.autoResize;
    snap.lineHeight = el.lineHeight;
  }

  if (isLinearLike(el)) {
    snap.points = el.points;
    snap.lastCommittedPoint = el.lastCommittedPoint;
    snap.startBinding = el.startBinding;
    snap.endBinding = el.endBinding;
    snap.startArrowhead = (el.startArrowhead as ExcalidrawArrowElement["startArrowhead"]) ?? null;
    snap.endArrowhead = (el.endArrowhead as ExcalidrawArrowElement["endArrowhead"]) ?? null;
    snap.pressures = el.pressures ?? null;
    snap.simulatePressure = el.simulatePressure;
  }

  if (isImageLike(el)) {
    snap.fileId = el.fileId ?? null;
    snap.status = el.status;
    snap.scale = el.scale;
    snap.crop = el.crop;
    const binary = getBinaryFileData(files, snap.fileId);
    snap.fileData = binary
      ? {
          mimeType: binary.mimeType,
          dataURL: binary.dataURL,
        }
      : null;
  }

  if (hasDataField(el)) {
    snap.data = el.data;
  }

  if (hasFrameId(el)) {
    snap.frameId = el.frameId ?? null;
  }

  if (el.type === "image") {
    snap.fileData = snap.fileData ?? null;
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
    frameId: hasFrameId(snap) ? snap.frameId ?? null : null,
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
      points: null as unknown as ExcalidrawLinearElement["points"],
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
      } as CanvasNode,
      nd,
      {},
      Date.now(),
    );
    if (!el || !isLinearLike(el) || !el.points || el.points.length < 2) {
      console.debug("[Canvas self-test] Freedraw normalization failed", el);
    }
  } catch (err) {
    console.debug("[Canvas self-test] Freedraw normalization threw", err);
  }
}

runFreedrawSelfTest();

function isNotFound(err: unknown): boolean {
  const message = err instanceof Error ? err.message : String(err);
  return message.toLowerCase().includes("not_found") || message.includes("404");
}

