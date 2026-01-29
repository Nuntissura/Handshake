import React, { useEffect, useMemo, useState } from "react";
import { SessionChatLogEntryV0_1, sessionChatGetSessionId, sessionChatRead } from "../../lib/sessionChat";

type Props = {
  isOpen: boolean;
  onClose: () => void;
};

const SHOW_INLINE_STORAGE_KEY = "handshake.ans001.showInline.v1";

function loadBooleanFromStorage(key: string, fallback: boolean): boolean {
  try {
    const raw = localStorage.getItem(key);
    if (raw === null) return fallback;
    return raw === "true";
  } catch {
    return fallback;
  }
}

export const Ans001TimelineDrawer: React.FC<Props> = ({ isOpen, onClose }) => {
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [entries, setEntries] = useState<SessionChatLogEntryV0_1[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedMessageId, setSelectedMessageId] = useState<string | null>(null);
  const [expandedMessageIds, setExpandedMessageIds] = useState<Set<string>>(new Set());
  const [showInline, setShowInline] = useState<boolean>(() => loadBooleanFromStorage(SHOW_INLINE_STORAGE_KEY, false));

  useEffect(() => {
    try {
      localStorage.setItem(SHOW_INLINE_STORAGE_KEY, showInline ? "true" : "false");
    } catch {
      // ignore localStorage failures
    }
  }, [showInline]);

  const refresh = async () => {
    setLoading(true);
    setError(null);
    try {
      const sid = await sessionChatGetSessionId();
      setSessionId(sid);
      const data = await sessionChatRead(sid, 400);
      setEntries(data);
      if (selectedMessageId && !data.some((e) => e.message_id === selectedMessageId)) {
        setSelectedMessageId(null);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load session chat log");
      setEntries([]);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (!isOpen) return;
    refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isOpen]);

  const newestFirst = useMemo(() => [...entries].reverse(), [entries]);

  const selected = useMemo(() => {
    if (!selectedMessageId) return null;
    return entries.find((e) => e.message_id === selectedMessageId) ?? null;
  }, [entries, selectedMessageId]);

  const toggleExpanded = (messageId: string) => {
    setExpandedMessageIds((prev) => {
      const next = new Set(prev);
      if (next.has(messageId)) next.delete(messageId);
      else next.add(messageId);
      return next;
    });
  };

  if (!isOpen) return null;

  return (
    <div className="ans001-drawer" role="dialog" aria-label="ANS-001 Timeline">
      <div className="drawer-header">
        <div>
          <p className="drawer-eyebrow">ANS-001</p>
          <h3 style={{ margin: 0 }}>Timeline</h3>
          <p className="muted small" style={{ margin: "6px 0 0 0" }}>
            Session: {sessionId ?? "unknown"}
          </p>
        </div>
        <div className="drawer-actions" style={{ marginTop: 0 }}>
          <button type="button" className="secondary" onClick={refresh} disabled={loading}>
            {loading ? "Loading..." : "Refresh"}
          </button>
          <button type="button" className="secondary" onClick={onClose}>
            Close
          </button>
        </div>
      </div>

      <div className="ans001-controls">
        <label className="ans001-toggle">
          <input type="checkbox" checked={showInline} onChange={(e) => setShowInline(e.target.checked)} />
          Show ANS-001 inline (default OFF)
        </label>
      </div>

      {error && <p className="error small">Error: {error}</p>}

      <div className="ans001-timeline">
        {newestFirst.length === 0 ? (
          <p className="muted">No session chat entries yet.</p>
        ) : (
          <ul className="ans001-timeline__list">
            {newestFirst.map((entry) => {
              const isSelected = entry.message_id === selectedMessageId;
              const hasAns001 = entry.role === "assistant" && entry.model_role === "frontend";
              return (
                <li key={entry.message_id}>
                  <button
                    type="button"
                    className={`ans001-timeline__row ${isSelected ? "selected" : ""}`}
                    onClick={() => setSelectedMessageId(entry.message_id)}
                  >
                    <div className="ans001-timeline__row-main">
                      <strong>{entry.role}</strong>
                      {hasAns001 && <span className="ans001-pill">ANS-001</span>}
                    </div>
                    <div className="ans001-timeline__row-meta">
                      <span>turn {entry.turn_index}</span>
                      <span>{entry.created_at_utc}</span>
                    </div>
                    <div className="ans001-timeline__row-content">{entry.content}</div>
                  </button>
                </li>
              );
            })}
          </ul>
        )}

        {selected && (
          <div className="ans001-detail">
            <h4 style={{ marginBottom: 6 }}>Selected message</h4>
            <div className="meta-list">
              <ul>
                <li>
                  <span className="muted">message_id</span> {selected.message_id}
                </li>
                <li>
                  <span className="muted">role</span> {selected.role}
                </li>
                {selected.role === "assistant" && (
                  <li>
                    <span className="muted">model_role</span> {selected.model_role ?? "null"}
                  </li>
                )}
              </ul>
            </div>
            <pre className="ans001-detail__content">{selected.content}</pre>

            {selected.role === "assistant" && selected.model_role === "frontend" && (
              <>
                {!showInline && (
                  <button type="button" className="secondary" onClick={() => toggleExpanded(selected.message_id)}>
                    {expandedMessageIds.has(selected.message_id) ? "Hide ANS-001" : "Show ANS-001"}
                  </button>
                )}
                {(showInline || expandedMessageIds.has(selected.message_id)) && (
                  <pre className="ans001-json">
                    {JSON.stringify(
                      {
                        ans001: selected.ans001 ?? null,
                        ans001_validation: selected.ans001_validation ?? null,
                      },
                      null,
                      2,
                    )}
                  </pre>
                )}
              </>
            )}
          </div>
        )}
      </div>
    </div>
  );
};

