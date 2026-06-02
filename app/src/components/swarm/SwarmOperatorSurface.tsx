import { useCallback, useEffect, useRef, useState } from "react";

import { Disclosure } from "../common/Disclosure";
import { SessionReplayPanel } from "../session/SessionReplayPanel";
import { TerminalPanel } from "../terminal/TerminalPanel";
import { SessionWorkbench } from "./SessionWorkbench";
import { SwarmBoard } from "./SwarmBoard";
import {
  SwarmResourceSection,
  SwarmSessionsSection,
  SwarmSpawnSection,
  swarmResourceBadge,
  useSwarmRoom,
} from "./SwarmControlRoom";
import { getSpawnTemplate } from "../../lib/ipc/swarm_runtime";

// SwarmOperatorSurface: the disclosure-organized operator surface for the
// multi-model swarm. Replaces the dense, always-expanded SwarmControlRoom wall
// rendered in the `swarm` pane. It owns the live swarm polling + spawn/cancel
// state via useSwarmRoom and lays the five dense blocks out as clearly-labelled
// collapsible <Disclosure> sections:
//
//   1. Swarm Board     — COLLAPSED by default (operator requirement). The heavy
//                        <SwarmBoard/> (and its live Tauri subscription + 10s
//                        reconcile loop) mounts LAZILY via Disclosure `lazy`:
//                        it is only mounted once the board disclosure is first
//                        opened, so a collapsed board holds no idle subscription.
//   2. Resource Budget — open by default (small glanceable status).
//   3. Spawn Session   — collapsed by default (an action form, not status).
//   4. Live Sessions   — open by default (primary operating context).
//   5. Operator Chat   — collapsed by default (tall panel, open on demand).

interface SwarmOperatorSurfaceProps {
  /** Initial open state for the Swarm Board disclosure (default false). App.tsx
   *  may pass a persisted operator preference here. Collapsed by default. */
  boardDefaultOpen?: boolean;
}

export function SwarmOperatorSurface({
  boardDefaultOpen = false,
}: SwarmOperatorSurfaceProps) {
  const room = useSwarmRoom();

  // Off-main-window terminal drawer wiring. The board's per-lane "Inspect
  // terminal" affordance knows a swarm_id; clicking it bumps `terminalOpenSignal`
  // (which force-opens the collapsed-by-default Terminal disclosure via the
  // Disclosure openSignal one-shot) and records the swarm_id to focus. The panel
  // resolves that swarm_id to its first captured session and selects its tab.
  // This is what makes the captured-background-work surface REACHABLE in the
  // product instead of shipping dark.
  const [terminalFocusSwarmId, setTerminalFocusSwarmId] = useState<string | null>(null);
  // Governance glue #3: the SessionWorkbench's "Show captured terminal" focuses
  // the SAME shared TerminalPanel by the selected chat session's composite
  // instance_id (the capture binding key). We track it separately from the
  // board's swarm-id focus and bump the same openSignal so there is exactly one
  // off-main-window terminal drawer (no second xterm instance).
  const [terminalFocusInstanceId, setTerminalFocusInstanceId] = useState<string | null>(null);
  const [terminalOpenSignal, setTerminalOpenSignal] = useState(0);
  const handleInspectTerminal = useCallback((swarmId: string) => {
    setTerminalFocusInstanceId(null);
    setTerminalFocusSwarmId(swarmId);
    setTerminalOpenSignal((n) => n + 1);
  }, []);
  const handleShowSessionTerminal = useCallback((instanceId: string) => {
    setTerminalFocusSwarmId(null);
    setTerminalFocusInstanceId(instanceId);
    setTerminalOpenSignal((n) => n + 1);
  }, []);

  // Off-main-window Session Replay drawer wiring. The board's per-card "Review
  // session" affordance knows a session's composite instance_id (the canonical
  // session id); clicking it bumps `replayOpenSignal` (force-opening the
  // collapsed-by-default Session Replay disclosure) and records the session id to
  // preselect. This makes the UNIFIED per-session audit record REACHABLE in the
  // product — the "go back and look when things go wrong" surface — instead of
  // shipping dark.
  const [replayFocusSessionId, setReplayFocusSessionId] = useState<string | null>(null);
  const [replayOpenSignal, setReplayOpenSignal] = useState(0);
  const handleReviewSession = useCallback((instanceId: string) => {
    setReplayFocusSessionId(instanceId);
    setReplayOpenSignal((n) => n + 1);
  }, []);

  // ROI #3 STATE RECOVERY wiring. Two convergent modes (per the design):
  //   - One-click resume (Session Replay row) -> focus the new session in the
  //     workbench. The Replay panel already drove the SAME validated spawn path
  //     and handed us the fresh composite; we just route focus + refresh tables.
  //   - Edit-then-resume (workbench button) -> read the stored template and
  //     PREFILL the existing Spawn form (force-opening its disclosure), so the
  //     operator can tweak before spawning. No new spawn logic either path.
  const [spawnOpenSignal, setSpawnOpenSignal] = useState(0);
  const [resumeNotice, setResumeNotice] = useState<string | null>(null);

  const handleResumed = useCallback(
    (newComposite: string) => {
      // Setting chatInstanceId focuses the workbench (chat + captured terminal +
      // transcript) via the existing sessionOpenSignal effect below — zero new
      // plumbing. refresh() pulls the new live session into the tables/board.
      room.setChatInstanceId(newComposite);
      void room.refresh();
    },
    [room],
  );

  const handleResumeViaForm = useCallback(
    async (instanceId: string) => {
      setResumeNotice(null);
      try {
        const tpl = await getSpawnTemplate(instanceId);
        if (!tpl) {
          // HONEST: no stored template => not resumable. Surface it; do not
          // fabricate a prefill.
          setResumeNotice(`Session ${instanceId} is not resumable (no spawn template stored).`);
          return;
        }
        room.prefillSpawnForm(tpl);
        setSpawnOpenSignal((n) => n + 1); // reveal the Spawn Session disclosure
      } catch (e) {
        setResumeNotice(e instanceof Error ? e.message : String(e));
      }
    },
    [room],
  );

  // Governance glue #3: when a session becomes the active chat session — via the
  // Live Sessions table "Chat" button, auto-select after spawn, or the picker —
  // force-open the collapsed-by-default "Session" workbench so the chat + its
  // captured terminal actually become visible. The openSignal is a one-shot
  // force-open (a no-op when the disclosure is already open; the operator can
  // still collapse it). Without this, clicking "Chat" silently selected a
  // session behind a collapsed panel.
  const [sessionOpenSignal, setSessionOpenSignal] = useState(0);
  const prevChatInstanceId = useRef<string | null>(null);
  useEffect(() => {
    const current = room.chatInstanceId;
    if (current && current !== prevChatInstanceId.current) {
      setSessionOpenSignal((n) => n + 1);
    }
    prevChatInstanceId.current = current;
  }, [room.chatInstanceId]);

  // Resource budget badge: in-use/cap, or an exhausted chip.
  let resourceBadge: string | undefined;
  if (room.snapshot.status === "ready") {
    const snap = room.snapshot.snapshot;
    resourceBadge = swarmResourceBadge(snap);
  }

  const sessionsCount =
    room.sessions.status === "ready" ? room.sessions.sessions.length : undefined;

  // Operator chat badge: short selected-session id, or "no session".
  const chatBadge = room.chatInstanceId
    ? room.chatInstanceId.length > 14
      ? `${room.chatInstanceId.slice(0, 14)}…`
      : room.chatInstanceId
    : "no session";

  return (
    <section
      className="swarm-operator-surface"
      data-testid="swarm-operator-surface"
      data-stable-id="swarm-operator-surface"
      aria-label="Swarm operator surface"
    >
      <Disclosure
        id="swarm-board"
        title="Swarm Board"
        defaultOpen={boardDefaultOpen}
        lazy
        data-testid="swarm-board-disclosure"
      >
        <SwarmBoard
          onInspectTerminal={handleInspectTerminal}
          onReviewSession={handleReviewSession}
        />
      </Disclosure>

      {/* Off-main-window integrated terminal drawer (WP-KERNEL-004). Collapsed
          by default + lazy via its own Disclosure host, so a closed panel costs
          nothing. Mounted here (NOT in the main window) so it is reachable on
          demand and the board's "Inspect terminal" affordance can reveal +
          focus a swarm's captured session. This is the "inspect all background
          work" capture surface. */}
      <TerminalPanel
        focusSwarmId={terminalFocusSwarmId}
        focusInstanceId={terminalFocusInstanceId}
        openSignal={terminalOpenSignal}
      />

      {/* Off-main-window unified Session Replay drawer (WP-KERNEL-004 governance
          glue #1). Collapsed-by-default + lazy via its own Disclosure host, so a
          closed panel costs nothing. Mounted here (NOT in the main window) so it
          is reachable on demand: the board's per-card "Review session" affordance
          reveals + preselects a session's UNIFIED record (chat + terminal + FR +
          process, one ordered timeline). This is the audit substrate for
          Handshake self-hosting this repo's governance. */}
      <SessionReplayPanel
        focusSessionId={replayFocusSessionId}
        openSignal={replayOpenSignal}
        onResumed={handleResumed}
      />

      {/* ROI #3: honest notice for the edit-then-resume (prefill) path — e.g. a
          session that turned out not to be resumable, or a get-template error. */}
      {resumeNotice ? (
        <p
          className="swarm-notice"
          data-testid="swarm-resume-notice"
          role="status"
          style={{ margin: "4px 0" }}
        >
          {resumeNotice}
        </p>
      ) : null}

      <Disclosure
        id="resource-budget"
        title="Resource Budget"
        count={resourceBadge}
        defaultOpen
        data-testid="resource-budget-disclosure"
      >
        <SwarmResourceSection snapshot={room.snapshot} />
      </Disclosure>

      <Disclosure
        id="spawn-session"
        title="Spawn Session"
        defaultOpen={false}
        openSignal={spawnOpenSignal}
        data-testid="spawn-session-disclosure"
      >
        <SwarmSpawnSection room={room} />
      </Disclosure>

      <Disclosure
        id="live-sessions"
        title="Live Sessions"
        count={sessionsCount}
        defaultOpen
        data-testid="live-sessions-disclosure"
      >
        <SwarmSessionsSection room={room} />
      </Disclosure>

      {/* The "Session" workbench (governance glue #3): chat (any provider) +
          a button to focus the shared TerminalPanel on this session's captured
          stdout + a button to open this session's full transcript. id and
          data-testid kept stable ("operator-chat") so existing tests/harnesses
          keep matching; the visible title reflects the broader chat+terminal+
          transcript scope. */}
      <Disclosure
        id="operator-chat"
        title="Session"
        count={chatBadge}
        defaultOpen={false}
        openSignal={sessionOpenSignal}
        data-testid="operator-chat-disclosure"
      >
        <SessionWorkbench
          room={room}
          onShowTerminal={handleShowSessionTerminal}
          onReviewSession={handleReviewSession}
          onResumeSession={handleResumeViaForm}
        />
      </Disclosure>
    </section>
  );
}
