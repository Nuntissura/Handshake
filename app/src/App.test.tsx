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
  getEvents: vi.fn(async () => []),
  createDiagnostic: vi.fn(async () => ({})),
}));

import { render, screen, waitFor } from "@testing-library/react";
import App from "./App";
import { FlightRecorderView } from "./components/FlightRecorderView";
import { createDiagnostic, getEvents, type FlightEvent } from "./lib/api";

it("renders desktop shell header and shows coordinator status", () => {
  render(<App />);

  expect(screen.getByText(/Desktop Shell/i)).toBeInTheDocument();
  expect(screen.getByTestId("system-status")).toBeInTheDocument();
});

describe("FlightRecorderView deep links", () => {
  let scrollIntoViewMock: ReturnType<typeof vi.fn>;

  const makeEvent = (eventId: string): FlightEvent => ({
    event_id: eventId,
    trace_id: "trace-1",
    timestamp: "2026-01-01T00:00:00Z",
    actor: "system",
    actor_id: "system",
    event_type: "system",
    wsids: [],
    payload: {},
  });

  beforeEach(() => {
    window.history.pushState({}, "", "/");
    scrollIntoViewMock = vi.fn();
    Object.defineProperty(HTMLElement.prototype, "scrollIntoView", {
      value: scrollIntoViewMock,
      writable: true,
      configurable: true,
    });

    vi.mocked(getEvents).mockClear();
    vi.mocked(createDiagnostic).mockClear();
  });

  it("focuses/selects event_id from URL by marking the row selected", async () => {
    const eventId = "evt-focus-1";
    window.history.pushState({}, "", `/?event_id=${encodeURIComponent(eventId)}`);

    vi.mocked(getEvents).mockResolvedValue([makeEvent(eventId)]);

    render(<FlightRecorderView />);

    const eventButton = await screen.findByRole("button", { name: eventId });
    const row = eventButton.closest("tr");
    expect(row).not.toBeNull();
    expect(row).toHaveClass("flight-recorder__row--selected");

    await waitFor(() => expect(scrollIntoViewMock).toHaveBeenCalled());
  });

  it("emits VAL-NAV-001 and shows a notice when event_id is not in returned results", async () => {
    const eventId = "evt-missing-1";
    window.history.pushState({}, "", `/?event_id=${encodeURIComponent(eventId)}`);

    vi.mocked(getEvents).mockResolvedValue([makeEvent("evt-other-1")]);

    render(<FlightRecorderView />);

    expect(
      await screen.findByText(/Event focus failed: event_id not present in returned events/i),
    ).toBeInTheDocument();

    await waitFor(() =>
      expect(vi.mocked(createDiagnostic)).toHaveBeenCalledWith(expect.objectContaining({ code: "VAL-NAV-001" })),
    );
  });
});
