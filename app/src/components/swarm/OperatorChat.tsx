import { useCallback, useEffect, useRef, useState } from "react";
import {
  chatGenerate,
  chatGenerateWithCloudEscalation,
  type CloudAssistanceReceiptContext,
  type SwarmEscalationTaskClass,
  type SwarmSession,
  type SwarmSpawnRequest,
} from "../../lib/ipc/swarm_runtime";

// OperatorChat: a REAL operator <-> model chat box. The operator picks ANY
// spawned session — local on-disk model, cloud BYOK, OR official-CLI bridge —
// and types; the message is sent via kernel_swarm_chat_generate to the live
// model and the returned tokens render. This is genuine generation through the
// swarm coordinator's session runtime, NOT a mock — the text comes from the
// real model's forward pass. The generate path is PROVIDER-AGNOSTIC backend-side
// (runtime.generate streams tokens for candle/llama, cloud BYOK, and the CLI
// bridge runtime identically), so this surface is no longer UI-scoped to local.
//
// Cloud output that runs through the explicit escalation lane stays
// receipt-gated: when a CloudAssistanceReceiptContext is supplied, the
// escalation passes it through so cloud assistance can be receipted.

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  return "Unknown error";
}

/**
 * Whether a session's lifecycle state can accept a chat generate turn. READY and
 * GENERATING sessions are live and chattable; QUEUED/LOADING are not ready yet,
 * and COMPLETED/FAILED/CANCELLED are terminal (the backend `resolve_runtime`
 * gate would refuse them). This is a UI COURTESY that mirrors the backend's real
 * gate — it never claims chattability the backend would refuse, and any backend
 * refusal (session reaped mid-turn, provider error) still renders verbatim via
 * the catch below. It is provider-INDEPENDENT: a cloud or CLI session in READY
 * is chattable, exactly like a local one.
 */
export function canChat(s: SwarmSession): boolean {
  return s.state === "READY" || s.state === "GENERATING";
}

/** Provider tag -> short operator-facing label (local / cloud / CLI). */
function providerLabel(provider: string): string {
  switch (provider) {
    case "local":
      return "local";
    case "byok_cloud":
      return "cloud";
    case "official_cli":
      return "CLI";
    default:
      return provider;
  }
}

/**
 * The source name shown in the picker: the local artifact's basename, else the
 * cloud model name, else a short model-id prefix. This is what lets the operator
 * tell apart sessions of the same provider.
 */
function sourceName(session: SwarmSession): string {
  if (session.artifactPath) {
    return session.artifactPath.split(/[/\\]/).pop() ?? session.artifactPath;
  }
  if (session.cloudModelName) return session.cloudModelName;
  return session.instanceId.modelId.slice(0, 8);
}

/**
 * The full provider-rich option label: "{provider} · {source}[ · wt:{worktree}]
 * (#{instance}, {state})". Puts provider + model + worktree in the picker so the
 * operator distinguishes local / cloud / CLI sessions at a glance.
 */
export function sessionOptionLabel(session: SwarmSession): string {
  const wt = session.worktreeId ? ` · wt:${session.worktreeId}` : "";
  return `${providerLabel(session.provider)} · ${sourceName(session)}${wt} (#${session.instanceId.instance}, ${session.state})`;
}

interface ChatTurn {
  role: "operator" | "model" | "system";
  text: string;
  // For model turns: how the generation finished + token count.
  finishReason?: string | null;
  tokenCount?: number;
}

export interface OperatorChatCloudEscalation {
  request: SwarmSpawnRequest;
  label: string;
  // Optional receipt context for receipt-gated cloud assistance. When present it
  // is passed to the escalation command so cloud output is receipted.
  receiptContext: CloudAssistanceReceiptContext | null;
}

interface OperatorChatProps {
  selectedInstanceId: string | null;
  /**
   * ALL spawned sessions (local, cloud BYOK, official CLI). The picker labels
   * each by provider + model + worktree; non-chattable (not READY/GENERATING)
   * sessions render as disabled options so the operator never selects a session
   * the backend would refuse.
   */
  sessions: SwarmSession[];
  onSelectInstance: (instanceId: string | null) => void;
  /**
   * Optional explicit cloud fallback lane. When supplied, the operator can opt
   * into local/VM-first generation with cloud fallback. The backend still owns
   * the real escalation decision; this prop only supplies the cloud lane request.
   */
  cloudEscalation?: OperatorChatCloudEscalation;
}

export function OperatorChat({
  selectedInstanceId,
  sessions,
  onSelectInstance,
  cloudEscalation,
}: OperatorChatProps) {
  const [prompt, setPrompt] = useState("");
  const [turns, setTurns] = useState<ChatTurn[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [escalationEnabled, setEscalationEnabled] = useState(false);
  const [taskClass, setTaskClass] = useState<SwarmEscalationTaskClass>("routine");
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

  const hasSessions = sessions.length > 0;
  // The currently-selected session object (if any), used to gate the composer on
  // its lifecycle state honestly.
  const selectedSession = selectedInstanceId
    ? sessions.find((s) => s.instanceId.composite === selectedInstanceId) ?? null
    : null;
  // A selected session is chattable only when live (READY/GENERATING). If it is
  // selected but not chattable, we disable the composer and explain why rather
  // than letting Send fail opaquely.
  const selectedChattable = selectedSession ? canChat(selectedSession) : false;
  const selectedCanEscalate =
    selectedChattable && selectedSession?.provider === "local";
  const cloudEscalationReady =
    !!cloudEscalation
    && cloudEscalation.request.provider !== "local"
    && !!cloudEscalation.request.cloudModelName?.trim();
  const escalationActive =
    escalationEnabled && selectedCanEscalate && cloudEscalationReady && !!selectedInstanceId;
  const composerDisabled = !selectedInstanceId || !selectedChattable;

  useEffect(() => {
    if (escalationEnabled && (!selectedCanEscalate || !cloudEscalationReady)) {
      setEscalationEnabled(false);
    }
  }, [cloudEscalationReady, escalationEnabled, selectedCanEscalate]);

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
      if (escalationActive && cloudEscalation) {
        const response = await chatGenerateWithCloudEscalation({
          localInstanceId: selectedInstanceId,
          prompt: text,
          cloud: cloudEscalation.request,
          taskClass,
          // Pass the receipt context through when supplied so receipt-gated
          // cloud assistance can be receipted. Optional on the backend command.
          cloudAssistanceReceipt: cloudEscalation.receiptContext,
        });
        if (response.escalated) {
          const receiptSuffix = response.cloudAssistanceReceipt
            ? ` (receipt ${response.cloudAssistanceReceipt.receiptId})`
            : "";
          setTurns((prev) => [
            ...prev,
            {
              role: "system",
              text: `Escalated to cloud: ${response.escalationReason ?? "cloud fallback selected"}${receiptSuffix}`,
            },
          ]);
        }
        const selectedResponse = response.cloud ?? response.local;
        if (!selectedResponse) {
          throw new Error(
            response.cloudError
              ?? response.localError
              ?? "cloud escalation produced no model response",
          );
        }
        setTurns((prev) => [
          ...prev,
          {
            role: "model",
            text:
              selectedResponse.text.length > 0
                ? selectedResponse.text
                : "(model produced no text)",
            finishReason: selectedResponse.finishReason,
            tokenCount: selectedResponse.tokenCount,
          },
        ]);
        return;
      }
      // REAL generate through the spawned session (any provider — the backend
      // generate path is provider-agnostic). Cloud output via the explicit
      // escalation lane above stays receipt-gated.
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
      // provider that errors on generate). Honest: we never swallow it.
      const message = errorMessage(err);
      setError(message);
      setTurns((prev) => [...prev, { role: "system", text: `Error: ${message}` }]);
    } finally {
      setBusy(false);
    }
  }, [cloudEscalation, escalationActive, prompt, selectedInstanceId, taskClass]);

  return (
    <section
      className="operator-chat"
      data-stable-id="operator-chat"
      data-testid="operator-chat"
      aria-labelledby="operator-chat-title"
    >
      <h3 id="operator-chat-title">Operator chat</h3>
      <p className="muted">
        Chat with a spawned model session (local, cloud, or CLI). Messages run a
        real generate through the swarm runtime; the rendered tokens are the
        model&apos;s actual output. Cloud output via the fallback lane runs
        through receipt-gated escalation.
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
              {hasSessions ? "Select a session..." : "No sessions spawned"}
            </option>
            {sessions.map((session) => {
              const composite = session.instanceId.composite;
              const chattable = canChat(session);
              return (
                <option
                  key={composite}
                  value={composite}
                  disabled={!chattable}
                  data-provider={session.provider}
                  data-testid={`operator-chat-option-${composite}`}
                >
                  {sessionOptionLabel(session)}
                </option>
              );
            })}
          </select>
        </label>
      </div>

      {/* Honest non-chattable note: when a session is selected but its lifecycle
          state cannot take a generate turn, say so explicitly instead of letting
          Send fail opaquely. */}
      {selectedSession && !selectedChattable ? (
        <p className="swarm-notice" data-testid="operator-chat-unsupported">
          Session is {selectedSession.state}; not ready for a chat turn.
        </p>
      ) : null}

      {cloudEscalation ? (
        <fieldset
          className="operator-chat__escalation"
          data-testid="operator-chat-escalation"
        >
          <legend>Cloud fallback</legend>
          <label className="operator-chat__escalation-toggle">
            <input
              type="checkbox"
              checked={escalationEnabled}
              disabled={!selectedCanEscalate || !cloudEscalationReady || busy}
              onChange={(event) => setEscalationEnabled(event.target.checked)}
              data-testid="operator-chat-escalation-enabled"
            />
            <span>Escalate local failures to {cloudEscalation.label}</span>
          </label>
          <label>
            <span>Task class</span>
            <select
              value={taskClass}
              disabled={!escalationEnabled || busy}
              onChange={(event) =>
                setTaskClass(event.target.value as SwarmEscalationTaskClass)
              }
              data-testid="operator-chat-escalation-task-class"
            >
              <option value="routine">routine</option>
              <option value="classification">classification</option>
              <option value="hard_reasoning">hard_reasoning</option>
              <option value="force_cloud">force_cloud</option>
              <option value="force_local">force_local</option>
            </select>
          </label>
          {!selectedCanEscalate && selectedSession ? (
            <p className="muted" data-testid="operator-chat-escalation-note">
              Select a READY local session to enable VM-local to cloud fallback.
            </p>
          ) : !cloudEscalationReady ? (
            <p className="muted" data-testid="operator-chat-escalation-note">
              Set a cloud model in the spawn form before enabling fallback.
            </p>
          ) : null}
        </fieldset>
      ) : null}

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
              : "Select a spawned session to start chatting."}
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
            !selectedInstanceId
              ? "Select a session first"
              : !selectedChattable
                ? `Session is ${selectedSession?.state ?? "not ready"}`
                : "Type a message to the model..."
          }
          disabled={composerDisabled || busy}
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
          disabled={composerDisabled || busy || prompt.trim().length === 0}
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
