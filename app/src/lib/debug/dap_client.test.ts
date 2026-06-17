// WP-KERNEL-009 MT-254 DebugAdapterCore — dap_client unit proofs.

import { afterEach, describe, expect, it, vi } from "vitest";
import {
  loadRunnableAdapters,
  loadBreakpoints,
  persistBreakpoints,
  toggleBreakpoint,
  parseDebugEvent,
} from "./dap_client";

function jsonResponse(body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status: 200,
    headers: { "content-type": "application/json" },
  });
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("MT-254 dap_client adapter honesty gate", () => {
  it("loads only runnable adapters (Node) and exposes no disabled entries", async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce(
      jsonResponse({
        adapters: [
          { kind: "node", id: "node", display_name: "Node.js (built-in inspector)", runnable: true },
        ],
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    const adapters = await loadRunnableAdapters();
    expect(adapters).toHaveLength(1);
    expect(adapters[0].id).toBe("node");
    expect(adapters.every((a) => a.runnable)).toBe(true);
    // Negative: no python/lldb leaks in.
    expect(adapters.some((a) => a.id === "python" || a.id === "lldb")).toBe(false);
    expect(fetchMock).toHaveBeenCalledWith(
      "http://127.0.0.1:37501/debug/adapters",
      expect.objectContaining({ method: "GET" }),
    );
  });

  it("refuses a non-runnable adapter (dead-entry guard)", async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce(
      jsonResponse({
        adapters: [{ kind: "node", id: "python", display_name: "Python", runnable: false }],
      }),
    );
    vi.stubGlobal("fetch", fetchMock);
    await expect(loadRunnableAdapters()).rejects.toThrow(/not runnable/);
  });
});

describe("MT-254 dap_client breakpoint persistence client", () => {
  it("loads and persists breakpoints through the durable backend route", async () => {
    const record = {
      breakpoint_id: "bp-1",
      rich_document_id: "doc-1",
      workspace_id: "w1",
      source_url: "file:///x.js",
      line: 2,
      condition: null,
      verified: true,
      updated_at: "2026-06-17T00:00:00Z",
      event_ledger_event_id: "KE-1",
    };
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(jsonResponse({ rich_document_id: "doc-1", breakpoints: [record] }))
      .mockResolvedValueOnce(jsonResponse({ rich_document_id: "doc-1", breakpoints: [record] }));
    vi.stubGlobal("fetch", fetchMock);

    const loaded = await loadBreakpoints("doc-1");
    expect(loaded[0].line).toBe(2);
    expect(loaded[0].verified).toBe(true);

    const stored = await persistBreakpoints("doc-1", "w1", [
      { source_url: "file:///x.js", line: 2 },
    ]);
    expect(stored[0].event_ledger_event_id).toBe("KE-1");
    expect(fetchMock).toHaveBeenNthCalledWith(
      2,
      "http://127.0.0.1:37501/debug/documents/doc-1/breakpoints",
      expect.objectContaining({ method: "PUT" }),
    );
  });
});

describe("MT-254 dap_client pure helpers", () => {
  it("toggles a breakpoint on and off at a line", () => {
    const empty = toggleBreakpoint([], "file:///x.js", 2);
    expect(empty).toEqual([{ source_url: "file:///x.js", line: 2 }]);
    const off = toggleBreakpoint(empty, "file:///x.js", 2);
    expect(off).toEqual([]);
    const other = toggleBreakpoint(empty, "file:///x.js", 7);
    expect(other).toHaveLength(2);
  });

  it("parses the typed dap event variants", () => {
    expect(parseDebugEvent({ kind: "stopped", reason: "breakpoint", top_frame_line: 2 })).toEqual({
      kind: "stopped",
      reason: "breakpoint",
      top_frame_line: 2,
      top_frame_source: undefined,
    });
    expect(parseDebugEvent({ kind: "terminated", exit_code: 0 })).toEqual({
      kind: "terminated",
      exit_code: 0,
    });
    expect(parseDebugEvent({ kind: "continued" })).toEqual({ kind: "continued" });
    expect(parseDebugEvent({ kind: "nonsense" })).toBeNull();
    expect(parseDebugEvent(null)).toBeNull();
  });
});
