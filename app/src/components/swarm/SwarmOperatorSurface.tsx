import { useCallback, useState } from "react";

import { Disclosure } from "../common/Disclosure";
import { SessionReplayPanel } from "../session/SessionReplayPanel";
import { TerminalPanel } from "../terminal/TerminalPanel";
import { OperatorChat } from "./OperatorChat";
import { SwarmBoard } from "./SwarmBoard";
import {
  SwarmResourceSection,
  SwarmSessionsSection,
  SwarmSpawnSection,
  useSwarmRoom,
} from "./SwarmControlRoom";

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
  const [terminalOpenSignal, setTerminalOpenSignal] = useState(0);
  const handleInspectTerminal = useCallback((swarmId: string) => {
    setTerminalFocusSwarmId(swarmId);
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

  // Resource budget badge: in-use/cap, or an exhausted chip.
  let resourceBadge: string | undefined;
  if (room.snapshot.status === "ready") {
    const snap = room.snapshot.snapshot;
    resourceBadge = snap.budgetExhausted
      ? "budget exhausted"
      : `${snap.concurrencyInUse}/${snap.concurrencyCap} in use`;
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
      />

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

      <Disclosure
        id="operator-chat"
        title="Operator Chat"
        count={chatBadge}
        defaultOpen={false}
        data-testid="operator-chat-disclosure"
      >
        <OperatorChat
          selectedInstanceId={room.chatInstanceId}
          localSessions={room.localSessions}
          onSelectInstance={room.setChatInstanceId}
        />
      </Disclosure>
    </section>
  );
}
