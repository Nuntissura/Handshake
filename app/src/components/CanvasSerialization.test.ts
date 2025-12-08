import { vi } from "vitest";
import { canvasToElements, elementsToGraph } from "./CanvasView";

vi.mock("@excalidraw/excalidraw", () => ({
  __esModule: true,
  Excalidraw: () => null,
}));

const baseMeta = {
  id: "canvas-1",
  workspace_id: "w1",
  title: "Canvas 1",
  created_at: "2025-01-01T00:00:00Z",
  updated_at: "2025-01-01T00:00:00Z",
};

describe("Canvas serialization helpers", () => {
  it("round-trips nodes and edges with key properties intact", () => {
    const nodes = [
      {
        id: "edge-1",
        kind: "arrow",
        position_x: 0,
        position_y: 0,
        data: {
          element: {
            id: "edge-1",
            type: "arrow",
            x: 0,
            y: 0,
            width: 10,
            height: 10,
            angle: 0,
            strokeColor: "#000000",
            backgroundColor: "#ffffff",
            fillStyle: "solid",
            strokeWidth: 1,
            strokeStyle: "solid",
            roundness: null,
            roughness: 0,
            opacity: 100,
            seed: 99,
            version: 1,
            versionNonce: 99,
            groupIds: [],
            boundElements: [],
            link: null,
            locked: false,
            isDeleted: false,
            points: [
              [0, 0],
              [5, 5],
            ],
            startArrowhead: null,
            endArrowhead: "arrow",
            startBinding: null,
            endBinding: null,
          },
        },
      },
      {
        id: "free-1",
        kind: "freedraw",
        position_x: 0,
        position_y: 0,
        data: {
          element: {
            id: "free-1",
            type: "freedraw",
            x: 0,
            y: 0,
            width: 10,
            height: 10,
            angle: 0,
            strokeColor: "#000000",
            backgroundColor: "#ffffff",
            fillStyle: "solid",
            strokeWidth: 1,
            strokeStyle: "solid",
            roundness: null,
            roughness: 1,
            opacity: 100,
            seed: 1,
            version: 1,
            versionNonce: 1,
            groupIds: [],
            boundElements: [],
            link: null,
            locked: false,
            isDeleted: false,
            points: [
              [0, 0],
              [5, 5],
              [10, 10],
            ],
            pressures: [0.5, 0.5, 0.5],
            simulatePressure: false,
          },
        },
      },
      {
        id: "rect-1",
        kind: "rectangle",
        position_x: 20,
        position_y: 20,
        data: {
          element: {
            id: "rect-1",
            type: "rectangle",
            x: 20,
            y: 20,
            width: 40,
            height: 30,
            angle: 0,
            strokeColor: "#111111",
            backgroundColor: "#eeeeee",
            fillStyle: "solid",
            strokeWidth: 1,
            strokeStyle: "solid",
            roundness: null,
            roughness: 0,
            opacity: 90,
            seed: 2,
            version: 1,
            versionNonce: 2,
            groupIds: [],
            boundElements: [],
            link: null,
            locked: false,
            isDeleted: false,
          },
        },
      },
    ];

    const edges = [
      {
        id: "edge-1",
        from_node_id: "free-1",
        to_node_id: "rect-1",
        kind: "arrow",
      },
    ];

    const { elements, files } = canvasToElements({ ...baseMeta, nodes, edges });
    const roundTrip = elementsToGraph(elements, files);

    const nodesById = new Map(roundTrip.nodes.map((n) => [n.id, n]));

    expect(nodesById.get("free-1")).toBeTruthy();
    expect(nodesById.get("rect-1")).toBeTruthy();

    const freeData = nodesById.get("free-1")?.data as { element: { points: number[][] } };
    expect(freeData?.element.points.length).toBe(3);
    expect(freeData?.element.points[2]).toEqual([10, 10]);

    const rectData = nodesById.get("rect-1")?.data as { element: { width: number; height: number; opacity: number } };
    expect(rectData?.element.width).toBe(40);
    expect(rectData?.element.height).toBe(30);
    expect(rectData?.element.opacity).toBe(90);

    expect(roundTrip.edges).toHaveLength(1);
    const [arrowEdge] = roundTrip.edges;
    expect(arrowEdge.from_node_id).toBe("free-1");
    expect(arrowEdge.to_node_id).toBe("rect-1");
  });
});
