import { Disclosure } from "../common/Disclosure";
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
        <SwarmBoard />
      </Disclosure>

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
