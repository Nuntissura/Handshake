import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";

import { Disclosure } from "../common/Disclosure";
import { TerminalView } from "./TerminalView";
import {
  defaultTerminalIpc,
  type CreateSessionRequest,
  type TerminalContext,
  type TerminalIpc,
  type TerminalSession,
} from "../../lib/ipc/terminal";

// WP-KERNEL-004 Integrated Terminal panel.
//
// Operator requirement: this is NOT in the main window. It is an off-main-window
// dockable/collapsible drawer hosted by the existing accessible Disclosure
// primitive, collapsed by default and lazy so a closed panel costs nothing (no
// xterm instances, no subscriptions). It is the "inspect all background work"
// capture surface: every captured session (swarm/sub-agent, cloud CLI bridge,
// MCP server, sandbox adapter) shows up here as a tab, grouped by swarm_id
// swimlane (the Jira board link), and is inspectable always and interactable on
// demand.
//
// LAW TERM-INVARIANTS is honored at the UI layer two ways:
//   1. AiJob (and any non-HumanDev) capture sessions render READ-ONLY by
//      default — inspect only. stdin is NOT wired.
//   2. To interact, the operator must (a) the backend must report the session
//      as interactiveAllowed (an interactive PTY that may request control), AND
//      (b) the operator must flip an explicit per-session "Take control" toggle.
//      Only then does the panel mount TerminalView with readOnly=false and wire
//      onData -> writeStdin.
// The toggle is UX intent, not authorization: kernel_terminal_write_stdin is the
// real enforcement boundary backend-side. The toggle merely refuses to even ask.
//
// The panel is structured so its rendering + tab logic + read-only/interact
// gating are unit-testable with the IPC client injected (Tauri `invoke` is
// unavailable under jsdom). Pass a fake `ipc` and a fake `renderTerminal` in
// tests; production uses defaultTerminalIpc and the real TerminalView.

export interface TerminalPanelProps {
  /** Injectable IPC client (tests pass a recording stub). */
  ipc?: TerminalIpc;
  /** Start expanded. Defaults to collapsed-by-default per operator requirement. */
  defaultOpen?: boolean;
  /**
   * Injectable terminal renderer so unit tests can avoid xterm (no canvas under
   * jsdom). Defaults to the real TerminalView.
   */
  renderTerminal?: (args: {
    session: TerminalSession;
    readOnly: boolean;
    ipc: TerminalIpc;
  }) => ReactNode;
  /** Optional: focus this session id when the panel mounts/opens (board link). */
  focusSessionId?: string | null;
  /**
   * Optional: focus the first captured session of this swarm when the panel
   * mounts/opens. The board's "Inspect terminal" affordance knows a swarm_id,
   * not a session_id, so this is the board → panel link. Re-applied whenever
   * `focusSignal` changes so repeat clicks re-focus.
   */
  focusSwarmId?: string | null;
  /**
   * Optional: focus the captured session whose source `instanceId` matches. The
   * swarm capture session for a model session is keyed by the swarm composite
   * instance_id (SessionBinding.instance_id), surfaced on
   * TerminalSession.instanceId. The SessionWorkbench knows the selected chat
   * session's composite instance_id (NOT a swarm_id or the capture session's own
   * id), so this is the honest binding path from a chat session to its captured
   * terminal tab. Resolved after focusSessionId and before focusSwarmId.
   */
  focusInstanceId?: string | null;
  /**
   * One-shot open driver forwarded to the host Disclosure. Bumping this opens
   * the drawer (board "Inspect terminal" → reveal the panel). Also used to
   * re-arm focus so clicking Inspect again re-focuses the swarm's lane.
   */
  openSignal?: number;
  /** Optional cwd for new/restarted HumanDev terminals. Should be the workspace root when known. */
  workspaceRoot?: string | null;
  /** Optional shell override for new/restarted HumanDev terminals. Backend default is used when omitted. */
  defaultShell?: string | null;
}

function laneLabel(session: TerminalSession): { key: string; label: string } {
  if (session.swarmId) return { key: `swarm:${session.swarmId}`, label: `swarm: ${session.swarmId}` };
  if (session.worktreeId) return { key: `wt:${session.worktreeId}`, label: `worktree: ${session.worktreeId}` };
  return { key: "ungrouped", label: "ungrouped" };
}

function defaultRenderTerminal({
  session,
  readOnly,
  ipc,
}: {
  session: TerminalSession;
  readOnly: boolean;
  ipc: TerminalIpc;
}): ReactNode {
  return (
    <TerminalView
      sessionId={session.sessionId}
      readOnly={readOnly}
      fetchScrollback={ipc.scrollback}
      subscribe={ipc.subscribe}
      onInput={readOnly ? undefined : (data) => void ipc.writeStdin(session.sessionId, data)}
      onResize={(cols, rows) => void ipc.resizeSession(session.sessionId, cols, rows)}
    />
  );
}

const TYPE_BADGE: Record<string, { label: string; bg: string }> = {
  HumanDev: { label: "human", bg: "#dbeafe" },
  AiJob: { label: "ai-job", bg: "#fef3c7" },
  PluginTool: { label: "plugin", bg: "#ede9fe" },
};

/** The inner panel body, only mounted once the disclosure is first opened. */
function TerminalPanelBody({
  ipc,
  renderTerminal,
  focusSessionId,
  focusSwarmId,
  focusInstanceId,
  focusSignal,
  workspaceRoot,
  defaultShell,
}: {
  ipc: TerminalIpc;
  renderTerminal: NonNullable<TerminalPanelProps["renderTerminal"]>;
  focusSessionId?: string | null;
  focusSwarmId?: string | null;
  focusInstanceId?: string | null;
  /** Bumped by the board so a repeat "Inspect terminal" re-arms focus. */
  focusSignal?: number;
  workspaceRoot?: string | null;
  defaultShell?: string | null;
}) {
  const [sessions, setSessions] = useState<TerminalSession[]>([]);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [terminalContext, setTerminalContext] = useState<TerminalContext | null>(null);
  // Per-session operator "Take control" intent. Absent/false = inspect-only.
  const [interacting, setInteracting] = useState<Record<string, boolean>>({});
  const [authorizing, setAuthorizing] = useState<Record<string, boolean>>({});
  // One-shot board focus guard, keyed by the current focus request. Each new
  // focusSignal re-arms it so a repeat Inspect click re-focuses the swarm lane.
  // setState only happens after an await inside the async refresh, so we never
  // call setState synchronously in an effect body.
  const focusedSignal = useRef<number | undefined>(undefined);

  const buildHumanSessionRequest = useCallback(
    (title: string, context: TerminalContext | null): CreateSessionRequest => {
      const request: CreateSessionRequest = {
        sessionType: "HumanDev",
        title,
      };
      const shell = (defaultShell ?? context?.defaultShell ?? "").trim();
      const cwd = (workspaceRoot ?? context?.cwd ?? "").trim();
      if (shell) request.shell = shell;
      if (cwd) request.cwd = cwd;
      return request;
    },
    [defaultShell, workspaceRoot],
  );

  const needsTerminalContext = !workspaceRoot?.trim() || !defaultShell?.trim();

  useEffect(() => {
    if (!needsTerminalContext) return;
    let active = true;
    void ipc.getContext()
      .then((context) => {
        if (active) setTerminalContext(context);
      })
      .catch(() => {
        // Non-fatal while merely opening the drawer; New/Restart surfaces the
        // error if context is still needed for a concrete terminal action.
      });
    return () => {
      active = false;
    };
  }, [ipc, needsTerminalContext]);

  const resolveHumanSessionRequest = useCallback(
    async (title: string): Promise<CreateSessionRequest> => {
      let context = terminalContext;
      if (needsTerminalContext && !context) {
        context = await ipc.getContext();
        setTerminalContext(context);
      }
      return buildHumanSessionRequest(title, context);
    },
    [buildHumanSessionRequest, ipc, needsTerminalContext, terminalContext],
  );

  const refresh = useCallback(async () => {
    try {
      const list = await ipc.listSessions();
      setSessions(list);
      setError(null);
      // Apply a one-shot board focus per focusSignal. We resolve the requested
      // session_id first, then fall back to the first session of the requested
      // swarm_id (the board affordance only knows a swarm_id). Re-arms whenever
      // focusSignal changes so repeated Inspect clicks re-focus.
      if (focusedSignal.current !== focusSignal) {
        const bySession = focusSessionId
          ? list.find((s) => s.sessionId === focusSessionId)
          : undefined;
        // Resolve by source instance_id (the SessionWorkbench knows the swarm
        // composite instance_id of the selected chat session, which the capture
        // binding stores on TerminalSession.instanceId) before the swarm-id
        // fallback. This is the chat-session → captured-terminal link.
        const byInstance = !bySession && focusInstanceId
          ? list.find((s) => s.instanceId === focusInstanceId)
          : undefined;
        const bySwarm = !bySession && !byInstance && focusSwarmId
          ? list.find((s) => s.swarmId === focusSwarmId)
          : undefined;
        const target = bySession ?? byInstance ?? bySwarm;
        if (target) {
          focusedSignal.current = focusSignal;
          setActiveId(target.sessionId);
          return;
        }
      }
      setActiveId((prev) => {
        if (prev && list.some((s) => s.sessionId === prev)) return prev;
        return list[0]?.sessionId ?? null;
      });
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [ipc, focusSessionId, focusSwarmId, focusInstanceId, focusSignal]);

  useEffect(() => {
    // refresh() awaits ipc.listSessions() BEFORE any setState, so this does not
    // synchronously update state in the effect body (mirrors SwarmBoard's
    // `void reconcile()`). The lint rule cannot see across the prop-method await
    // boundary, so it is disabled here with that rationale.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    void refresh();
  }, [refresh]);

  const lanes = useMemo(() => {
    const map = new Map<string, { label: string; sessions: TerminalSession[] }>();
    for (const s of sessions) {
      const { key, label } = laneLabel(s);
      let lane = map.get(key);
      if (!lane) {
        lane = { label, sessions: [] };
        map.set(key, lane);
      }
      lane.sessions.push(s);
    }
    return [...map.entries()]
      .map(([key, v]) => ({ key, ...v }))
      .sort((a, b) => a.label.localeCompare(b.label));
  }, [sessions]);

  const active = sessions.find((s) => s.sessionId === activeId) ?? null;

  const handleNewSession = useCallback(async () => {
    try {
      const created = await ipc.createSession(await resolveHumanSessionRequest("Terminal"));
      setSessions((prev) => {
        if (prev.some((s) => s.sessionId === created.sessionId)) {
          return prev.map((s) => (s.sessionId === created.sessionId ? created : s));
        }
        return [...prev, created];
      });
      setActiveId(created.sessionId);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [ipc, resolveHumanSessionRequest]);

  const handleCloseSession = useCallback(async () => {
    if (!active) return;
    if (active.sessionType !== "HumanDev") return;
    const closingId = active.sessionId;
    try {
      await ipc.closeSession(closingId);
      const next = sessions.filter((s) => s.sessionId !== closingId);
      setSessions(next);
      setInteracting((prev) => {
        const copy = { ...prev };
        delete copy[closingId];
        return copy;
      });
      setActiveId(next[0]?.sessionId ?? null);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [active, ipc, sessions]);

  const handleRestartSession = useCallback(async () => {
    if (!active || active.sessionType !== "HumanDev") return;
    const restarting = active;
    try {
      await ipc.closeSession(restarting.sessionId);
      const remaining = sessions.filter((s) => s.sessionId !== restarting.sessionId);
      setSessions(remaining);
      setInteracting((prev) => {
        const copy = { ...prev };
        delete copy[restarting.sessionId];
        return copy;
      });
      setActiveId(remaining[0]?.sessionId ?? null);

      const created = await ipc.createSession(await resolveHumanSessionRequest(restarting.title || "Terminal"));
      setSessions((prev) => [...prev.filter((s) => s.sessionId !== created.sessionId), created]);
      setActiveId(created.sessionId);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [active, ipc, resolveHumanSessionRequest, sessions]);

  const handleTakeControlChange = useCallback(
    async (session: TerminalSession, checked: boolean) => {
      if (!session.interactiveAllowed || session.exited) return;
      if (!checked) {
        setInteracting((prev) => ({ ...prev, [session.sessionId]: false }));
        setError(null);
        return;
      }
      setAuthorizing((prev) => ({ ...prev, [session.sessionId]: true }));
      try {
        await ipc.authorizeInteractive(session.sessionId);
        setInteracting((prev) => ({ ...prev, [session.sessionId]: true }));
        setError(null);
      } catch (e) {
        setInteracting((prev) => ({ ...prev, [session.sessionId]: false }));
        setError(e instanceof Error ? e.message : String(e));
      } finally {
        setAuthorizing((prev) => ({ ...prev, [session.sessionId]: false }));
      }
    },
    [ipc],
  );

  // A session is read-only unless it is interactive-capable AND the operator is
  // in control of it. HumanDev sessions are interactive by default (their whole
  // purpose), so they have no Take-control gate while the PTY is live. Exited
  // sessions remain inspectable but never writable.
  // Non-HumanDev (AiJob/PluginTool) sessions default to read-only (inspect) and
  // become interactive only after the operator flips Take control AND the
  // backend reports interactiveAllowed (TERM-INVARIANTS).
  const isReadOnly = (s: TerminalSession): boolean => {
    if (s.exited) return true;
    if (!s.interactiveAllowed) return true; // backend capability gate (authority)
    if (s.sessionType === "HumanDev") return false; // human terminal: interactive
    return !interacting[s.sessionId]; // capture session: gated by Take control
  };

  return (
    <div className="terminal-panel" data-testid="terminal-panel-body">
      <div
        className="terminal-panel__actions"
        style={{ display: "flex", gap: 6, alignItems: "center", justifyContent: "flex-end", marginBottom: 8 }}
      >
        <button
          type="button"
          data-testid="terminal-new-session"
          aria-label="New terminal session"
          onClick={() => void handleNewSession()}
          style={{
            fontSize: 12,
            padding: "3px 8px",
            borderRadius: 6,
            border: "1px solid var(--hs-color-border, #d1d5db)",
            background: "var(--hs-color-surface)",
            color: "var(--hs-color-text)",
          }}
        >
          New
        </button>
      </div>
      <div
        className="terminal-panel__tabs"
        data-testid="terminal-panel-tabs"
        role="tablist"
        aria-label="Terminal sessions"
        style={{ display: "flex", flexDirection: "column", gap: 6, marginBottom: 8 }}
      >
        {error && (
          <div data-testid="terminal-panel-error" style={{ color: "#dc2626", fontSize: 12 }}>
            Terminal IPC error: {error}
          </div>
        )}
        {sessions.length === 0 && !error && (
          <div data-testid="terminal-panel-empty" style={{ color: "var(--hs-color-text-subtle)", fontSize: 13 }}>
            No captured terminal sessions. Background work (swarms, cloud CLI, MCP, sandboxes) appears here when captured.
          </div>
        )}
        {lanes.map((lane) => (
          <div key={lane.key} data-testid={`terminal-lane-${lane.key}`} style={{ display: "flex", gap: 6, alignItems: "center", flexWrap: "wrap" }}>
            <span style={{ fontSize: 11, color: "var(--hs-color-text-subtle)", fontWeight: 600 }}>{lane.label}</span>
            {lane.sessions.map((s) => {
              const badge = TYPE_BADGE[s.sessionType] ?? { label: s.sessionType, bg: "#eee" };
              const selected = s.sessionId === activeId;
              return (
                <button
                  key={s.sessionId}
                  type="button"
                  role="tab"
                  aria-selected={selected}
                  className={`terminal-tab${selected ? " terminal-tab--active" : ""}`}
                  data-testid={`terminal-tab-${s.sessionId}`}
                  data-active={selected ? "true" : "false"}
                  onClick={() => setActiveId(s.sessionId)}
                  style={{
                    fontSize: 12,
                    padding: "3px 8px",
                    borderRadius: 6,
                    border: selected ? "1px solid #2563eb" : "1px solid var(--hs-color-border, #d1d5db)",
                    background: selected ? "#eff6ff" : "var(--hs-color-surface)",
                    color: "var(--hs-color-text)",
                    display: "flex",
                    gap: 6,
                    alignItems: "center",
                  }}
                >
                  <span style={{ fontSize: 10, padding: "0 5px", borderRadius: 8, background: badge.bg, color: "#111" }}>
                    {badge.label}
                  </span>
                  <span style={{ maxWidth: 160, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                    {s.title || s.sessionId}
                  </span>
                  {s.exited && (
                    <span data-testid={`terminal-tab-exited-${s.sessionId}`} style={{ fontSize: 10, color: "#6b7280" }}>
                      exited{s.exitCode === null ? "" : ` (${s.exitCode})`}
                    </span>
                  )}
                </button>
              );
            })}
          </div>
        ))}
      </div>

      {active && (
        <div className="terminal-panel__active" data-testid="terminal-panel-active" data-active-session={active.sessionId}>
          <div
            className="terminal-panel__toolbar"
            style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 6, fontSize: 12 }}
          >
            <strong style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
              {active.title || active.sessionId}
            </strong>
            {isReadOnly(active) ? (
              <span data-testid="terminal-readonly-badge" style={{ color: "#6b7280" }}>
                read-only (inspect)
              </span>
            ) : (
              <span data-testid="terminal-interactive-badge" style={{ color: "#16a34a" }}>
                interactive
              </span>
            )}
            <div style={{ display: "flex", gap: 6, marginLeft: "auto" }}>
              <button
                type="button"
                data-testid="terminal-restart-session"
                aria-label="Restart terminal session"
                disabled={active.sessionType !== "HumanDev"}
                onClick={() => void handleRestartSession()}
                style={{
                  fontSize: 12,
                  padding: "3px 8px",
                  borderRadius: 6,
                  border: "1px solid var(--hs-color-border, #d1d5db)",
                  background: "var(--hs-color-surface)",
                  color: "var(--hs-color-text)",
                  opacity: active.sessionType === "HumanDev" ? 1 : 0.5,
                }}
              >
                Restart
              </button>
              <button
                type="button"
                data-testid="terminal-close-session"
                aria-label="Close terminal session"
                disabled={active.sessionType !== "HumanDev"}
                onClick={() => void handleCloseSession()}
                style={{
                  fontSize: 12,
                  padding: "3px 8px",
                  borderRadius: 6,
                  border: "1px solid var(--hs-color-border, #d1d5db)",
                  background: "var(--hs-color-surface)",
                  color: "var(--hs-color-text)",
                  opacity: active.sessionType === "HumanDev" ? 1 : 0.5,
                }}
              >
                Close
              </button>
            </div>

            {/* Take-control gate. Only offered when the backend reports the
                session interactive-capable. For AiJob capture sessions that the
                backend has NOT granted, the control is honestly disabled — never
                faked — preserving TERM-INVARIANTS at the UI surface. */}
            {active.sessionType !== "HumanDev" && (
              <label
                data-testid="terminal-take-control"
                title={
                  active.interactiveAllowed
                    ? "Take control: wire keyboard input to this session"
                    : "Interaction not permitted for this session (no capability grant)"
                }
                style={{ display: "flex", gap: 4, alignItems: "center", opacity: active.interactiveAllowed && !active.exited ? 1 : 0.5 }}
              >
                <input
                  type="checkbox"
                  data-testid="terminal-take-control-checkbox"
                  disabled={!active.interactiveAllowed || active.exited || !!authorizing[active.sessionId]}
                  checked={!active.exited && !!interacting[active.sessionId] && active.interactiveAllowed}
                  onChange={(e) => void handleTakeControlChange(active, e.target.checked)}
                />
                <span>Take control</span>
              </label>
            )}
          </div>

          {/* Keyed by sessionId+readOnly so flipping Take-control remounts the
              view and (un)wires stdin cleanly — never hot-swaps the stdin
              binding under a live xterm. */}
          <div className="terminal-panel__surface" style={{ border: "1px solid var(--hs-color-border, #d1d5db)", borderRadius: 6, overflow: "hidden", background: "#000" }}>
            {renderTerminal({
              session: active,
              readOnly: isReadOnly(active),
              ipc,
            })}
          </div>
        </div>
      )}
    </div>
  );
}

/**
 * The off-main-window terminal drawer. Collapsed-by-default + lazy: nothing in
 * the body (sessions list, xterm, subscriptions) mounts until the operator first
 * opens it.
 */
export function TerminalPanel({
  ipc = defaultTerminalIpc,
  defaultOpen = false,
  renderTerminal = defaultRenderTerminal,
  focusSessionId,
  focusSwarmId,
  focusInstanceId,
  openSignal,
  workspaceRoot,
  defaultShell,
}: TerminalPanelProps) {
  return (
    <Disclosure
      id="terminal"
      title="Terminal"
      defaultOpen={defaultOpen}
      lazy
      openSignal={openSignal}
      data-testid="terminal-panel"
    >
      <TerminalPanelBody
        ipc={ipc}
        renderTerminal={renderTerminal}
        focusSessionId={focusSessionId}
        focusSwarmId={focusSwarmId}
        focusInstanceId={focusInstanceId}
        focusSignal={openSignal}
        workspaceRoot={workspaceRoot}
        defaultShell={defaultShell}
      />
    </Disclosure>
  );
}
