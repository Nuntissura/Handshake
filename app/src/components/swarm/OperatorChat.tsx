import { useCallback, useEffect, useRef, useState } from "react";
import {
  chatGenerate,
  type SwarmSession,
} from "../../lib/ipc/swarm_runtime";

// OperatorChat: a REAL operator <-> local-model chat box. The operator picks a
// spawned local-model session and types; the message is sent via
// kernel_swarm_chat_generate to the live model and the returned tokens render.
// This is genuine generation through the swarm coordinator's session runtime,
// NOT a mock — the text comes from the real model's forward pass.

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  return "Unknown error";
}

interface ChatTurn {
  role: "operator" | "model" | "system";
  text: string;
  // For model turns: how the generation finished + token count.
  finishReason?: string | null;
  tokenCount?: number;
}

interface OperatorChatProps {
  selectedInstanceId: string | null;
  localSessions: SwarmSession[];
  onSelectInstance: (instanceId: string | null) => void;
}

export function OperatorChat({
  selectedInstanceId,
  localSessions,
  onSelectInstance,
}: OperatorChatProps) {
  const [prompt, setPrompt] = useState("");
  const [turns, setTurns] = useState<ChatTurn[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const logRef = useRef<HTMLDivElement>(null);

  // Reset the transcript when the operator switches sessions (each session is a
  // distinct model instance with its own context).
  useEffect(() => {
    setTurns([]);
    setError(null);
  }, [selectedInstanceId]);

  useEffect(() => {
    if (logRef.current) {
      logRef.current.scrollTop = logRef.current.scrollHeight;
    }
  }, [turns]);

  const handleSend = useCallback(async () => {
    const text = prompt.trim();
    if (!text || !selectedInstanceId) {
      return;
    }
    setError(null);
    setBusy(true);
    setTurns((prev) => [...prev, { role: "operator", text }]);
    setPrompt("");
    try {
      // REAL generate through the spawned local-model session.
      const response = await chatGenerate(selectedInstanceId, text);
      setTurns((prev) => [
        ...prev,
        {
          role: "model",
          text: response.text.length > 0 ? response.text : "(model produced no text)",
          finishReason: response.finishReason,
          tokenCount: response.tokenCount,
        },
      ]);
    } catch (err) {
      // Surface the REAL backend error verbatim (session reaped, busy instance,
      // generate failure).
      const message = errorMessage(err);
      setError(message);
      setTurns((prev) => [...prev, { role: "system", text: `Error: ${message}` }]);
    } finally {
      setBusy(false);
    }
  }, [prompt, selectedInstanceId]);

  const hasLocalSessions = localSessions.length > 0;

  return (
    <section
      className="operator-chat"
      data-stable-id="operator-chat"
      data-testid="operator-chat"
      aria-labelledby="operator-chat-title"
    >
      <h3 id="operator-chat-title">Operator chat</h3>
      <p className="muted">
        Chat with a spawned local-model session. Messages run a real generate through
        the swarm runtime; the rendered tokens are the model&apos;s actual output.
      </p>

      <div className="operator-chat__session-picker">
        <label>
          <span>Session</span>
          <select
            value={selectedInstanceId ?? ""}
            data-testid="operator-chat-session"
            onChange={(event) => onSelectInstance(event.target.value || null)}
          >
            <option value="">
              {hasLocalSessions ? "Select a local session..." : "No local sessions spawned"}
            </option>
            {localSessions.map((session) => {
              const composite = session.instanceId.composite;
              const label = session.artifactPath
                ? session.artifactPath.split(/[/\\]/).pop()
                : composite;
              return (
                <option key={composite} value={composite}>
                  {label} (#{session.instanceId.instance}, {session.state})
                </option>
              );
            })}
          </select>
        </label>
      </div>

      <div
        className="operator-chat__log"
        ref={logRef}
        data-testid="operator-chat-log"
        aria-live="polite"
      >
        {turns.length === 0 ? (
          <p className="muted" data-testid="operator-chat-empty">
            {selectedInstanceId
              ? "No messages yet. Type below to chat with the model."
              : "Select a spawned local session to start chatting."}
          </p>
        ) : (
          turns.map((turn, index) => (
            <div
              key={index}
              className={`operator-chat__turn operator-chat__turn--${turn.role}`}
              data-testid={`operator-chat-turn-${turn.role}-${index}`}
              data-role={turn.role}
            >
              <span className="operator-chat__role">{turn.role}</span>
              <span className="operator-chat__text">{turn.text}</span>
              {turn.role === "model" && (turn.tokenCount !== undefined) ? (
                <span className="operator-chat__meta muted">
                  {turn.tokenCount} tokens
                  {turn.finishReason ? ` · ${turn.finishReason}` : ""}
                </span>
              ) : null}
            </div>
          ))
        )}
      </div>

      <form
        className="operator-chat__composer"
        data-testid="operator-chat-composer"
        onSubmit={(event) => {
          event.preventDefault();
          void handleSend();
        }}
      >
        <textarea
          value={prompt}
          rows={2}
          placeholder={
            selectedInstanceId
              ? "Type a message to the model..."
              : "Select a session first"
          }
          disabled={!selectedInstanceId || busy}
          data-testid="operator-chat-input"
          onChange={(event) => setPrompt(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && !event.shiftKey) {
              event.preventDefault();
              void handleSend();
            }
          }}
        />
        <button
          type="submit"
          disabled={!selectedInstanceId || busy || prompt.trim().length === 0}
          data-testid="operator-chat-send"
        >
          {busy ? "Generating..." : "Send"}
        </button>
      </form>

      {error ? (
        <p className="swarm-error" data-testid="operator-chat-error">
          {error}
        </p>
      ) : null}
    </section>
  );
}
