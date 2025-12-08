import { render, screen, waitFor } from "@testing-library/react";
import { vi } from "vitest";
import { WorkspaceSidebar } from "./WorkspaceSidebar";

vi.mock("../lib/api", () => {
  const listWorkspaces = vi.fn();
  const listDocuments = vi.fn();
  const listCanvases = vi.fn();
  return {
    listWorkspaces,
    listDocuments,
    listCanvases,
    createWorkspace: vi.fn(),
    createDocument: vi.fn(),
    createCanvas: vi.fn(),
    __esModule: true,
  };
});

describe("WorkspaceSidebar", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const noop = () => {};

  it("renders workspaces on successful fetch without error banner", async () => {
    const api = await import("../lib/api");
    api.listDocuments.mockResolvedValue([]);
    api.listCanvases.mockResolvedValue([]);
    api.listWorkspaces.mockResolvedValue([
      { id: "w1", name: "Workspace 1", created_at: "", updated_at: "" },
    ]);

    render(
      <WorkspaceSidebar
        onSelectDocument={noop}
        onSelectCanvas={noop}
        selectedDocumentId={null}
        selectedCanvasId={null}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText("Workspace 1")).toBeInTheDocument();
    });
    expect(screen.queryByText(/Could not refresh the workspace list/i)).not.toBeInTheDocument();
  });

  it("shows error banner and Retry button when fetch fails", async () => {
    const api = await import("../lib/api");
    api.listDocuments.mockResolvedValue([]);
    api.listCanvases.mockResolvedValue([]);
    api.listWorkspaces.mockRejectedValue(new Error("network down"));

    render(
      <WorkspaceSidebar
        onSelectDocument={noop}
        onSelectCanvas={noop}
        selectedDocumentId={null}
        selectedCanvasId={null}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText(/Could not refresh the workspace list/i)).toBeInTheDocument();
    });
    expect(screen.getByRole("button", { name: /retry/i })).toBeInTheDocument();
  });
});
