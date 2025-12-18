import { render, screen, waitFor } from "@testing-library/react";
import { vi } from "vitest";
import type { DebugEvent } from "../state/debugEvents";

const mockApi = () =>
  vi.doMock("../lib/api", () => ({
    getLogTail: vi.fn(async () => ({ lines: ["log line 1"] })),
    getHealth: vi.fn(async () => ({ status: "ok", db_status: "ok" })),
  }));

async function loadPanelWithEvents(events: DebugEvent[]) {
  vi.resetModules();
  mockApi();
  vi.doMock("../state/debugEvents", () => ({
    subscribeDebugEvents: (listener: (evts: DebugEvent[]) => void) => {
      listener(events);
      return () => {};
    },
  }));
  const { DebugPanel } = await import("./DebugPanel");
  return DebugPanel;
}

describe("DebugPanel", () => {
  it("renders healthy system status and recent events", async () => {
    const Panel = await loadPanelWithEvents([
      { id: "1", type: "doc-save", targetId: "doc-1", result: "ok", ts: 0 },
      { id: "2", type: "canvas-load", targetId: "canvas-1", result: "ok", ts: 0 },
    ]);
    render(<Panel />);

    expect(screen.getByText(/Debug \/ Status/i)).toBeInTheDocument();

    await waitFor(() => {
      expect(screen.getByText(/Health/i, { selector: "span" })).toBeInTheDocument();
      expect(screen.getAllByText(/ok/i).length).toBeGreaterThanOrEqual(2); // health + db
      expect(screen.getByText(/DB/i)).toBeInTheDocument();
    });

    await waitFor(() => {
      expect(screen.getByText(/doc-save/i)).toBeInTheDocument();
      expect(screen.getByText(/canvas-load/i)).toBeInTheDocument();
    });
  });

  it("renders unhealthy system status when health fails", async () => {
    vi.resetModules();
    vi.doMock("../lib/api", () => ({
      getLogTail: vi.fn(async () => ({ lines: [] })),
      getHealth: vi.fn(async () => ({ status: "error", db_status: "down" })),
    }));
    vi.doMock("../state/debugEvents", () => ({
      subscribeDebugEvents: (listener: (evts: DebugEvent[]) => void) => {
        listener([]);
        return () => {};
      },
    }));
    const { DebugPanel } = await import("./DebugPanel");
    render(<DebugPanel />);

    await waitFor(() => {
      expect(screen.getByText(/error/i)).toBeInTheDocument();
      expect(screen.getByText(/down/i)).toBeInTheDocument();
    });
  });

  it("shows placeholder when no events exist", async () => {
    const Panel = await loadPanelWithEvents([]);
    render(<Panel />);
    await waitFor(() => expect(screen.getByText(/No recent events yet/i)).toBeInTheDocument());
  });
});
