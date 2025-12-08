import { vi } from "vitest";

vi.mock("@excalidraw/excalidraw", () => ({
  __esModule: true,
  Excalidraw: () => null,
}));

vi.mock("./components/WorkspaceSidebar", () => ({
  WorkspaceSidebar: () => <div data-testid="workspace-sidebar">Workspace Sidebar</div>,
}));

vi.mock("./components/SystemStatus", () => ({
  SystemStatus: () => <div data-testid="system-status">Coordinator: OK</div>,
}));

vi.mock("./lib/api", () => ({
  listWorkspaces: vi.fn(async () => [
    {
      id: "w1",
      name: "Workspace 1",
      created_at: "2025-01-01T00:00:00Z",
      updated_at: "2025-01-01T00:00:00Z",
    },
  ]),
  createWorkspace: vi.fn(async (name: string) => ({
    id: "w-new",
    name,
    created_at: "2025-01-01T00:00:00Z",
    updated_at: "2025-01-01T00:00:00Z",
  })),
  listDocuments: vi.fn(async () => []),
  createDocument: vi.fn(async (workspaceId: string, title: string) => ({
    id: "doc-1",
    workspace_id: workspaceId,
    title,
    created_at: "2025-01-01T00:00:00Z",
    updated_at: "2025-01-01T00:00:00Z",
  })),
  listCanvases: vi.fn(async () => []),
  createCanvas: vi.fn(async (workspaceId: string, title: string) => ({
    id: "canvas-1",
    workspace_id: workspaceId,
    title,
    created_at: "2025-01-01T00:00:00Z",
    updated_at: "2025-01-01T00:00:00Z",
  })),
  getDocument: vi.fn(async () => ({
    id: "doc-1",
    workspace_id: "w1",
    title: "Doc 1",
    created_at: "2025-01-01T00:00:00Z",
    updated_at: "2025-01-01T00:00:00Z",
    blocks: [],
  })),
  updateDocumentBlocks: vi.fn(async () => []),
  getCanvas: vi.fn(async (id: string) => ({
    id,
    workspace_id: "w1",
    title: "Canvas 1",
    created_at: "2025-01-01T00:00:00Z",
    updated_at: "2025-01-01T00:00:00Z",
    nodes: [],
    edges: [],
  })),
  updateCanvasGraph: vi.fn(async (canvasId: string) => ({
    id: canvasId,
    workspace_id: "w1",
    title: "Canvas 1",
    created_at: "2025-01-01T00:00:00Z",
    updated_at: "2025-01-01T00:00:00Z",
    nodes: [],
    edges: [],
  })),
}));

import { render, screen } from "@testing-library/react";
import App from "./App";

it("renders desktop shell header and shows coordinator status", () => {
  render(<App />);

  expect(screen.getByText(/Desktop Shell/i)).toBeInTheDocument();
  expect(screen.getByTestId("system-status")).toBeInTheDocument();
});
