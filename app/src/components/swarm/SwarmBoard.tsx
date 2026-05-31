import { useCallback, useEffect, useRef, useState } from "react";

import {
  BOARD_COLUMNS,
  boardSnapshot,
  cancelSession,
  cardKey,
  eventTargetState,
  subscribeBoardEvents,
  type SwarmBoardCard,
  type SwarmBoardDelta,
} from "../../lib/ipc/swarm_runtime";

// rank-4: the live operator board (Jira-style). The board is a READ-MODEL
// PROJECTION of the coordinator's SwarmEvents -- columns = ModelSessionState,
// swimlanes = the swarm/worktree grouping, cards = sessions. It fetches a
// snapshot on mount + on resync, then applies pushed `swarm://event` deltas in
// place; a `seq` gap or a `swarm://resync` triggers a full reconcile so the
// board never drifts on dropped events. UI writes are COMMANDS (cancel/escalate),
// never direct column mutations. Replaces the 1500ms poll (kept only as a slow,
// visibility-gated reconcile safety net).

interface BoardState {
  cards: Record<string, SwarmBoardCard>;
}

function indexCards(cards: SwarmBoardCard[]): Record<string, SwarmBoardCard> {
  const out: Record<string, SwarmBoardCard> = {};
  for (const c of cards) out[cardKey(c.instanceId)] = c;
  return out;
}

const BOOTING = new Set(["QUEUED", "LOADING"]);

/** Live board state + drift-safe delta application. */
export function useSwarmBoard() {
  const [board, setBoard] = useState<BoardState>({ cards: {} });
  const [lagged, setLagged] = useState(false);
  // null = no delta baseline yet (just reconciled); the next delta sets it.
  const seqRef = useRef<number | null>(null);
  const reconcileRef = useRef<(() => Promise<void>) | null>(null);

  const reconcile = useCallback(async () => {
    const snap = await boardSnapshot();
    seqRef.current = null; // re-baseline against the next pushed delta
    setBoard({ cards: indexCards(snap.cards) });
    setLagged(false);
  }, []);
  reconcileRef.current = reconcile;

  const applyDelta = useCallback((delta: SwarmBoardDelta) => {
    // A gap in the monotonic seq means we missed events -> reconcile, never
    // apply a partial stream blind (the single biggest board-correctness risk).
    if (seqRef.current !== null && delta.seq !== seqRef.current + 1) {
      setLagged(true);
      void reconcileRef.current?.();
      return;
    }
    seqRef.current = delta.seq;

    const target = eventTargetState(delta.event);
    if (target === null) {
      // SessionSpawned introduces a new card whose full data isn't in the event.
      if ("SessionSpawned" in delta.event) {
        setLagged(true);
        void reconcileRef.current?.();
      }
      return;
    }
    setBoard((b) => {
      const card = b.cards[target.key];
      if (!card) {
        void reconcileRef.current?.(); // unknown card -> reconcile to learn it
        return b;
      }
      return { cards: { ...b.cards, [target.key]: { ...card, state: target.state } } };
    });
  }, []);

  useEffect(() => {
    let alive = true;
    let unlisten: (() => void) | undefined;
    void reconcile();
    void subscribeBoardEvents(applyDelta, () => {
      setLagged(true);
      void reconcile();
    }).then((u) => {
      if (alive) unlisten = u;
      else u();
    });
    // Slow reconcile safety net (Prefect WS-down pattern) -- NOT a 1500ms poll.
    const timer = window.setInterval(() => {
      if (document.visibilityState === "visible") void reconcile();
    }, 10_000);
    return () => {
      alive = false;
      unlisten?.();
      window.clearInterval(timer);
    };
  }, [reconcile, applyDelta]);

  return { board, lagged, reconcile };
}

interface Lane {
  key: string;
  label: string;
  byState: Record<string, SwarmBoardCard[]>;
  total: number;
}

function laneKeyOf(card: SwarmBoardCard): { key: string; label: string } {
  if (card.swarmId) return { key: `swarm:${card.swarmId}`, label: `swarm: ${card.swarmId}` };
  if (card.worktreeId) return { key: `wt:${card.worktreeId}`, label: `worktree: ${card.worktreeId}` };
  return { key: "ungrouped", label: "ungrouped" };
}

function groupIntoLanes(cards: SwarmBoardCard[]): Lane[] {
  const lanes = new Map<string, Lane>();
  for (const card of cards) {
    const { key, label } = laneKeyOf(card);
    let lane = lanes.get(key);
    if (!lane) {
      lane = { key, label, byState: {}, total: 0 };
      lanes.set(key, lane);
    }
    (lane.byState[card.state] ??= []).push(card);
    lane.total += 1;
  }
  return [...lanes.values()].sort((a, b) => a.label.localeCompare(b.label));
}

const STATE_COLOR: Record<string, string> = {
  QUEUED: "#6b7280",
  LOADING: "#d97706",
  READY: "#2563eb",
  GENERATING: "#16a34a",
  COMPLETED: "#15803d",
  FAILED: "#dc2626",
  CANCELLED: "#6b7280",
};

function SwarmCard({ card, onCancel }: { card: SwarmBoardCard; onCancel: () => void }) {
  const terminal = card.state === "COMPLETED" || card.state === "FAILED" || card.state === "CANCELLED";
  return (
    <div
      className="swarm-card"
      style={{
        border: `1px solid ${STATE_COLOR[card.state] ?? "#888"}`,
        borderLeftWidth: 4,
        borderRadius: 6,
        padding: "6px 8px",
        marginBottom: 6,
        background: "var(--card-bg, #fff)",
        fontSize: 12,
      }}
      title={`${card.instanceId.composite} · ${card.provider} · ${card.runtimeBinding}`}
    >
      <div style={{ fontWeight: 600, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
        {card.instanceId.modelId.slice(0, 8)}#{card.instanceId.instance}
      </div>
      <div style={{ display: "flex", gap: 6, alignItems: "center", marginTop: 2 }}>
        <span
          style={{
            fontSize: 10,
            padding: "0 5px",
            borderRadius: 8,
            background: card.provider === "local" ? "#e0e7ff" : "#fef3c7",
          }}
        >
          {card.provider}
        </span>
        {card.worktreeId && <span style={{ fontSize: 10, color: "#6b7280" }}>wt:{card.worktreeId}</span>}
      </div>
      {!terminal && (
        <button onClick={onCancel} style={{ marginTop: 4, fontSize: 10 }}>
          Cancel
        </button>
      )}
    </div>
  );
}

/** The live swarm operator board. */
export function SwarmBoard() {
  const { board, lagged, reconcile } = useSwarmBoard();
  const cards = Object.values(board.cards);
  const lanes = groupIntoLanes(cards);
  const booting = cards.filter((c) => BOOTING.has(c.state)).length;
  const running = cards.filter((c) => c.state === "GENERATING").length;

  return (
    <div className="swarm-board" data-testid="swarm-board">
      <header style={{ display: "flex", gap: 12, alignItems: "center", marginBottom: 8 }}>
        <strong>Swarm board</strong>
        <span style={{ fontSize: 12, color: "#6b7280" }}>
          {booting} booting · {running} running · {cards.length} total
        </span>
        {lagged && (
          <span style={{ fontSize: 12, color: "#d97706" }} data-testid="board-resync-banner">
            Resyncing…
          </span>
        )}
        <button onClick={() => void reconcile()} style={{ marginLeft: "auto", fontSize: 12 }}>
          Refresh
        </button>
      </header>

      {lanes.length === 0 && (
        <div style={{ color: "#6b7280", fontSize: 13, padding: 12 }}>No active sessions.</div>
      )}

      {lanes.map((lane) => (
        <section key={lane.key} style={{ marginBottom: 16 }}>
          <h4 style={{ margin: "4px 0", fontSize: 13 }}>
            {lane.label} <span style={{ color: "#6b7280", fontWeight: 400 }}>({lane.total})</span>
          </h4>
          <div
            className="board-columns"
            style={{ display: "grid", gridTemplateColumns: `repeat(${BOARD_COLUMNS.length}, 1fr)`, gap: 8 }}
          >
            {BOARD_COLUMNS.map((col) => (
              <div key={col} className="board-column" style={{ minWidth: 0 }}>
                <div
                  style={{
                    fontSize: 11,
                    fontWeight: 600,
                    color: STATE_COLOR[col],
                    borderBottom: `2px solid ${STATE_COLOR[col]}`,
                    marginBottom: 6,
                    paddingBottom: 2,
                  }}
                >
                  {col} ({lane.byState[col]?.length ?? 0})
                </div>
                {(lane.byState[col] ?? []).map((card) => (
                  <SwarmCard
                    key={cardKey(card.instanceId)}
                    card={card}
                    onCancel={() => void cancelSession(card.instanceId.composite)}
                  />
                ))}
              </div>
            ))}
          </div>
        </section>
      ))}
    </div>
  );
}
