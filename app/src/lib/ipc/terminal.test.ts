import { beforeEach, describe, expect, it, vi } from "vitest";

const invokeMock = vi.hoisted(() => vi.fn());
const listenMock = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: listenMock,
}));

import {
  createSession,
  decodeChunk,
  getContext,
  getDiagnostics,
  listSessions,
  runCommand,
  scrollback,
  subscribe,
  writeStdin,
  authorizeInteractive,
} from "./terminal";

// These tests target the pure, jsdom-safe parts of the terminal IPC client:
// the base64 -> raw bytes decode that feeds xterm. The Tauri `invoke` / `listen`
// wrappers are exercised by the Playwright real-app path, not here (no Tauri
// runtime under jsdom). The decode is the correctness-critical seam: terminal
// output carries ANSI control sequences and multibyte UTF-8 that MUST round-trip
// byte-for-byte; stringifying corrupts both.

function toBase64(bytes: Uint8Array): string {
  let binary = "";
  for (const b of bytes) binary += String.fromCharCode(b);
  return btoa(binary);
}

describe("decodeChunk", () => {
  it("round-trips ASCII bytes exactly", () => {
    const bytes = new Uint8Array([0x68, 0x69, 0x0a]); // "hi\n"
    expect(Array.from(decodeChunk(toBase64(bytes)))).toEqual([0x68, 0x69, 0x0a]);
  });

  it("preserves ANSI control sequences without corruption", () => {
    // ESC [ 3 1 m  (red), bytes 0x1b 0x5b 0x33 0x31 0x6d
    const ansi = new Uint8Array([0x1b, 0x5b, 0x33, 0x31, 0x6d, 0x41, 0x1b, 0x5b, 0x30, 0x6d]);
    expect(Array.from(decodeChunk(toBase64(ansi)))).toEqual(Array.from(ansi));
  });

  it("preserves multibyte UTF-8 bytes (e.g. 'é' = 0xc3 0xa9) byte-for-byte", () => {
    const utf8 = new Uint8Array([0xc3, 0xa9]);
    const out = decodeChunk(toBase64(utf8));
    expect(out.length).toBe(2);
    expect(out[0]).toBe(0xc3);
    expect(out[1]).toBe(0xa9);
  });

  it("round-trips INVALID UTF-8 bytes without loss or replacement chars", () => {
    // 0xff 0xfe are not valid UTF-8 lead bytes; a TextDecoder path would mangle
    // these into U+FFFD. The byte decode must keep them exactly.
    const invalid = new Uint8Array([0xff, 0xfe, 0x00, 0x80, 0xc0]);
    const out = decodeChunk(toBase64(invalid));
    expect(Array.from(out)).toEqual([0xff, 0xfe, 0x00, 0x80, 0xc0]);
  });

  it("decodes an empty chunk to an empty buffer", () => {
    expect(decodeChunk("").length).toBe(0);
  });

  it("returns a Uint8Array (never a string)", () => {
    expect(decodeChunk(toBase64(new Uint8Array([1, 2, 3])))).toBeInstanceOf(Uint8Array);
  });
});

describe("terminal Tauri IPC binding", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    listenMock.mockReset();
  });

  it("creates a session using the backend req envelope and normalizes Rust session info", async () => {
    invokeMock.mockResolvedValueOnce({
      sessionId: "term-1",
      kind: "INTERACTIVE",
      sessionType: "HUMAN_DEV",
      swarmId: "swarm-1",
      worktreeId: "wt-1",
      instanceId: "inst-1",
      traceId: "trace-1",
      title: null,
      interactiveAuthorized: true,
    });

    const result = await createSession({
      sessionType: "HumanDev",
      shell: "pwsh",
      args: ["-NoLogo"],
      cwd: "D:/repo",
      rows: 30,
      cols: 100,
      swarmId: "swarm-1",
      worktreeId: "wt-1",
      instanceId: "inst-1",
      title: null,
      capabilityScope: ["terminal.interact"],
    });

    expect(invokeMock).toHaveBeenCalledWith("kernel_terminal_create_session", {
      req: {
        sessionType: "HUMAN_DEV",
        shell: "pwsh",
        args: ["-NoLogo"],
        cwd: "D:/repo",
        rows: 30,
        cols: 100,
        swarmId: "swarm-1",
        worktreeId: "wt-1",
        instanceId: "inst-1",
        title: null,
        capabilityScope: ["terminal.interact"],
      },
    });
    expect(result).toEqual({
      sessionId: "term-1",
      sessionType: "HumanDev",
      swarmId: "swarm-1",
      worktreeId: "wt-1",
      instanceId: "inst-1",
      title: "term-1",
      exited: false,
      exitCode: null,
      interactiveAllowed: true,
    });
  });

  it("lists sessions and converts backend enum casing for capture sessions", async () => {
    invokeMock.mockResolvedValueOnce([
      {
        sessionId: "term-ai",
        kind: "CAPTURE",
        sessionType: "AI_JOB",
        swarmId: null,
        worktreeId: null,
        instanceId: "agent-1",
        traceId: "trace-ai",
        title: "Agent output",
        interactiveAuthorized: false,
      },
    ]);

    const result = await listSessions();

    expect(invokeMock).toHaveBeenCalledWith("kernel_terminal_list_sessions");
    expect(result).toEqual([
      {
        sessionId: "term-ai",
        sessionType: "AiJob",
        swarmId: null,
        worktreeId: null,
        instanceId: "agent-1",
        title: "Agent output",
        exited: false,
        exitCode: null,
        interactiveAllowed: false,
      },
    ]);
  });

  it("keeps Take-control reachable for an interactive AiJob before authorization", async () => {
    invokeMock.mockResolvedValueOnce([
      {
        sessionId: "term-ai-live",
        kind: "INTERACTIVE",
        sessionType: "AI_JOB",
        swarmId: "swarm-a",
        worktreeId: "wt-a",
        instanceId: "agent-a",
        traceId: "trace-live",
        title: "Agent live shell",
        interactiveAuthorized: false,
      },
    ]);

    const result = await listSessions();

    expect(result).toEqual([
      {
        sessionId: "term-ai-live",
        sessionType: "AiJob",
        swarmId: "swarm-a",
        worktreeId: "wt-a",
        instanceId: "agent-a",
        title: "Agent live shell",
        exited: false,
        exitCode: null,
        interactiveAllowed: true,
      },
    ]);
  });

  it("gets backend-resolved terminal context", async () => {
    invokeMock.mockResolvedValueOnce({ cwd: "D:/repo", defaultShell: null });

    await expect(getContext()).resolves.toEqual({ cwd: "D:/repo", defaultShell: null });
    expect(invokeMock).toHaveBeenCalledWith("kernel_terminal_context");
  });

  it("gets runtime-wide terminal diagnostics", async () => {
    invokeMock.mockResolvedValueOnce({ receiptFailureCount: 2 });

    await expect(getDiagnostics()).resolves.toEqual({ receiptFailureCount: 2 });
    expect(invokeMock).toHaveBeenCalledWith("kernel_terminal_diagnostics");
  });

  it("fails closed when the backend returns an unknown session type", async () => {
    invokeMock.mockResolvedValueOnce([
      {
        sessionId: "term-unknown",
        kind: "CAPTURE",
        sessionType: "REMOTE_AGENT",
        swarmId: null,
        worktreeId: null,
        instanceId: "remote-1",
        traceId: "trace-unknown",
        title: "remote",
      },
    ]);

    const result = await listSessions();

    expect(result).toEqual([
      {
        sessionId: "term-unknown",
        sessionType: "PluginTool",
        swarmId: null,
        worktreeId: null,
        instanceId: "remote-1",
        title: "remote",
        exited: false,
        exitCode: null,
        interactiveAllowed: false,
      },
    ]);
  });

  it("writes stdin as base64 bytes with an explicit human-author flag", async () => {
    invokeMock.mockResolvedValueOnce(undefined);

    await writeStdin("term-1", "é\n");

    expect(invokeMock).toHaveBeenCalledWith("kernel_terminal_write_stdin", {
      sessionId: "term-1",
      dataBase64: toBase64(new TextEncoder().encode("é\n")),
      asAi: false,
    });
  });

  it("authorizes interactive control through the backend capability gate", async () => {
    invokeMock.mockResolvedValueOnce(undefined);

    await authorizeInteractive("term-ai");

    expect(invokeMock).toHaveBeenCalledWith("kernel_terminal_authorize_interactive", {
      sessionId: "term-ai",
    });
  });

  it("wraps the backend scrollback base64 string in the frontend snapshot shape", async () => {
    const chunkBase64 = toBase64(new TextEncoder().encode("hello"));
    invokeMock.mockResolvedValueOnce(chunkBase64);

    await expect(scrollback("term-1")).resolves.toEqual({
      sessionId: "term-1",
      seq: 0,
      chunkBase64,
      truncated: false,
    });
    expect(invokeMock).toHaveBeenCalledWith("kernel_terminal_scrollback", {
      sessionId: "term-1",
    });
  });

  it("runs one-shot commands using the backend req envelope and decodes outputBase64", async () => {
    const outputBase64 = toBase64(new TextEncoder().encode("ok\r\n"));
    invokeMock.mockResolvedValueOnce({
      sessionId: "term-run",
      exitCode: 0,
      timedOut: false,
      outputBase64,
    });

    const result = await runCommand({
      command: "pwsh",
      args: ["-NoLogo", "-Command", "Write-Output ok"],
      cwd: "D:/repo",
      timeoutMs: 1234,
      swarmId: "swarm-1",
      capabilityScope: ["terminal.interact"],
    });

    expect(invokeMock).toHaveBeenCalledWith("kernel_terminal_run_command", {
      req: {
        shell: "pwsh",
        args: ["-NoLogo", "-Command", "Write-Output ok"],
        cwd: "D:/repo",
        timeoutMs: 1234,
        swarmId: "swarm-1",
        capabilityScope: ["terminal.interact"],
      },
    });
    expect(result).toEqual({
      sessionId: "term-run",
      exitCode: 0,
      output: "ok\r\n",
      timedOut: false,
    });
  });

  it("uses the backend explicit timedOut flag for one-shot commands", async () => {
    const outputBase64 = toBase64(new TextEncoder().encode("\u001b[6n"));
    invokeMock.mockResolvedValueOnce({
      sessionId: "term-timeout",
      exitCode: -1,
      timedOut: true,
      outputBase64,
    });

    const result = await runCommand({
      command: "cmd.exe",
      args: ["/C", "echo", "never-reached"],
      timeoutMs: 25,
    });

    expect(result).toEqual({
      sessionId: "term-timeout",
      exitCode: -1,
      output: "\u001b[6n",
      timedOut: true,
    });
  });

  it("subscribes to camelCase backend event payloads", async () => {
    const listeners = new Map<string, (event: { payload: unknown }) => void>();
    const unlisten = vi.fn();
    listenMock.mockImplementation(async (eventName: string, handler: (event: { payload: unknown }) => void) => {
      listeners.set(eventName, handler);
      return unlisten;
    });
    const onOutput = vi.fn();
    const onExit = vi.fn();
    const onResync = vi.fn();

    const unsubscribe = await subscribe({ onOutput, onExit, onResync });

    listeners.get("terminal://output")?.({
      payload: {
        sessionId: "term-1",
        seq: 1,
        chunkBase64: toBase64(new Uint8Array([1, 2, 3])),
      },
    });
    listeners.get("terminal://exit")?.({
      payload: {
        sessionId: "term-1",
        exitCode: 7,
      },
    });
    listeners.get("terminal://resync")?.({
      payload: {
        sessionId: "term-1",
        dropped: 4,
      },
    });
    unsubscribe();

    expect(onOutput).toHaveBeenCalledWith("term-1", new Uint8Array([1, 2, 3]));
    expect(onExit).toHaveBeenCalledWith("term-1", 7);
    expect(onResync).toHaveBeenCalledWith("term-1", { reason: "broadcast-lag", dropped: 4 });
    expect(unlisten).toHaveBeenCalledTimes(3);
  });
});
