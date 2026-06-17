import { useEffect, useMemo, useState } from "react";
import { openDailyJournal, type LoomBlock } from "../lib/api";

type Props = {
  workspaceId: string;
  initialDate?: string;
  todayDate?: string;
};

type JournalLoadState = {
  workspaceId: string;
  journalDate: string;
  status: "loading" | "ready" | "error";
  block: LoomBlock | null;
  error: string | null;
};

function localToday(): string {
  const now = new Date();
  const year = now.getFullYear();
  const month = String(now.getMonth() + 1).padStart(2, "0");
  const day = String(now.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function addDays(date: string, days: number): string {
  const next = new Date(`${date}T00:00:00Z`);
  next.setUTCDate(next.getUTCDate() + days);
  return next.toISOString().slice(0, 10);
}

function blockTitle(block: LoomBlock | null, journalDate: string): string {
  return block?.title?.trim() || `Daily Note ${journalDate}`;
}

export function LoomDailyJournalPanel({ workspaceId, initialDate, todayDate }: Props) {
  const today = useMemo(() => todayDate ?? localToday(), [todayDate]);
  const [journalDate, setJournalDate] = useState(initialDate ?? today);
  const [loadState, setLoadState] = useState<JournalLoadState>({
    workspaceId,
    journalDate: initialDate ?? today,
    status: "loading",
    block: null,
    error: null,
  });

  useEffect(() => {
    let cancelled = false;
    const requestedWorkspaceId = workspaceId;
    const requestedDate = journalDate;
    openDailyJournal(workspaceId, journalDate)
      .then((nextBlock) => {
        if (!cancelled) {
          setLoadState({
            workspaceId: requestedWorkspaceId,
            journalDate: requestedDate,
            status: "ready",
            block: nextBlock,
            error: null,
          });
        }
      })
      .catch((err) => {
        if (!cancelled) {
          setLoadState({
            workspaceId: requestedWorkspaceId,
            journalDate: requestedDate,
            status: "error",
            block: null,
            error: err instanceof Error ? err.message : "Daily journal unavailable",
          });
        }
      });
    return () => {
      cancelled = true;
    };
  }, [workspaceId, journalDate]);

  const isCurrentRequest = loadState.workspaceId === workspaceId && loadState.journalDate === journalDate;
  const loading = !isCurrentRequest || loadState.status === "loading";
  const error = isCurrentRequest && loadState.status === "error" ? loadState.error : null;
  const block = isCurrentRequest && loadState.status === "ready" ? loadState.block : null;

  return (
    <div className="content-card loom-block-panel loom-daily-journal-panel" data-testid="loom-daily-journal-panel">
      <header className="loom-block-panel__header loom-daily-journal-panel__header">
        <div>
          <p className="app-eyebrow">Daily Journal</p>
          <h2 data-testid="loom-daily-journal-panel.title">{blockTitle(block, journalDate)}</h2>
        </div>
        <span className="kernel-dcc__badge" data-testid="loom-daily-journal-panel.date">
          {journalDate}
        </span>
      </header>
      <div className="loom-daily-journal-panel__toolbar" aria-label="Daily journal navigation">
        <button
          type="button"
          data-testid="loom-daily-journal-panel.prev"
          onClick={() => setJournalDate((current) => addDays(current, -1))}
        >
          Previous
        </button>
        <button type="button" data-testid="loom-daily-journal-panel.today" onClick={() => setJournalDate(today)}>
          Today
        </button>
        <button
          type="button"
          data-testid="loom-daily-journal-panel.next"
          onClick={() => setJournalDate((current) => addDays(current, 1))}
        >
          Next
        </button>
      </div>
      {loading ? <p>Loading daily journal...</p> : null}
      {error ? <p className="error">{error}</p> : null}
      {!loading && !error && block ? (
        <>
          <dl className="loom-block-panel__facts">
            <div>
              <dt>Block</dt>
              <dd>{block.block_id}</dd>
            </div>
            <div>
              <dt>Links</dt>
              <dd>
                {block.derived.backlink_count} backlinks / {block.derived.mention_count} mentions /{" "}
                {block.derived.tag_count} tags
              </dd>
            </div>
          </dl>
          {block.derived.full_text_index ? (
            <section className="loom-block-panel__body">
              <h3>Indexed Text</h3>
              <p>{block.derived.full_text_index}</p>
            </section>
          ) : null}
        </>
      ) : null}
    </div>
  );
}
