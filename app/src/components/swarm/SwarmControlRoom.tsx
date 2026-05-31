import { useCallback, useEffect, useRef, useState } from "react";
import {
  cancelSession,
  listActiveSessions,
  resourceSnapshot,
  spawnSession,
  type SwarmProvider,
  type SwarmResourceSnapshot,
  type SwarmRuntimeBinding,
  type SwarmSession,
  type SwarmSpawnRequest,
} from "../../lib/ipc/swarm_runtime";
import { OperatorChat } from "./OperatorChat";

// SwarmControlRoom: the real operator surface for the multi-model swarm. Polls
// the live coordinator (list + resource snapshot), spawns local artifacts or
// cloud models through the REAL backend, and cancels sessions. Errors are
// surfaced verbatim from the backend (e.g. PROVIDER_NOT_CONFIGURED).
//
// The dense blocks (resource bar, spawn form, sessions table, operator chat)
// are extracted into reusable presentational sub-components + a `useSwarmRoom`
// orchestration hook so SwarmOperatorSurface can lay each one out inside its
// own collapsible <Disclosure>. SwarmControlRoom itself remains a working
// (non-disclosure) composition for any caller / test that still mounts it.

export const POLL_INTERVAL_MS = 1500;

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  return "Unknown error";
}

export type SessionsState =
  | { status: "loading" }
  | { status: "error"; message: string }
  | { status: "ready"; sessions: SwarmSession[] };

export type SnapshotState =
  | { status: "loading" }
  | { status: "error"; message: string }
  | { status: "ready"; snapshot: SwarmResourceSnapshot };

/**
 * Orchestration hook owning the live swarm polling + spawn/cancel state. Shared
 * by SwarmControlRoom and SwarmOperatorSurface so both surfaces drive the same
 * real coordinator backend with identical behavior.
 */
export function useSwarmRoom() {
  const [sessions, setSessions] = useState<SessionsState>({ status: "loading" });
  const [snapshot, setSnapshot] = useState<SnapshotState>({ status: "loading" });

  // Spawn form state.
  const [provider, setProvider] = useState<SwarmProvider>("local");
  const [runtimeBinding, setRuntimeBinding] = useState<SwarmRuntimeBinding>("candle");
  const [artifactPath, setArtifactPath] = useState("");
  const [sha256, setSha256] = useState("");
  const [cloudModelName, setCloudModelName] = useState("");
  const [instance, setInstance] = useState(0);
  const [spawning, setSpawning] = useState(false);
  const [spawnError, setSpawnError] = useState<string | null>(null);
  const [spawnNotice, setSpawnNotice] = useState<string | null>(null);

  // Selected session for the operator chat box (composite instance id).
  const [chatInstanceId, setChatInstanceId] = useState<string | null>(null);

  const pollRef = useRef<number | null>(null);

  const refresh = useCallback(async () => {
    try {
      const rows = await listActiveSessions();
      setSessions({ status: "ready", sessions: rows });
    } catch (error) {
      setSessions({ status: "error", message: errorMessage(error) });
    }
    try {
      const snap = await resourceSnapshot();
      setSnapshot({ status: "ready", snapshot: snap });
    } catch (error) {
      setSnapshot({ status: "error", message: errorMessage(error) });
    }
  }, []);

  useEffect(() => {
    let active = true;
    const tick = async () => {
      if (!active) return;
      await refresh();
    };
    void tick();
    pollRef.current = window.setInterval(() => {
      void tick();
    }, POLL_INTERVAL_MS);
    return () => {
      active = false;
      if (pollRef.current !== null) {
        window.clearInterval(pollRef.current);
        pollRef.current = null;
      }
    };
  }, [refresh]);

  const handleSpawn = useCallback(async () => {
    setSpawnError(null);
    setSpawnNotice(null);
    setSpawning(true);
    try {
      const request: SwarmSpawnRequest = { provider, instance };
      if (provider === "local") {
        request.runtimeBinding = runtimeBinding;
        request.artifactPath = artifactPath.trim();
        request.sha256Expected = sha256.trim();
      } else {
        request.cloudModelName = cloudModelName.trim();
      }
      const id = await spawnSession(request);
      setSpawnNotice(`Spawned session ${id.composite}`);
      // Auto-select the new local session for the chat box.
      if (provider === "local") {
        setChatInstanceId(id.composite);
      }
      await refresh();
    } catch (error) {
      // Surface the REAL backend error verbatim (e.g. PROVIDER_NOT_CONFIGURED,
      // FactoryFailed sha mismatch, missing artifact).
      setSpawnError(errorMessage(error));
    } finally {
      setSpawning(false);
    }
  }, [
    provider,
    runtimeBinding,
    artifactPath,
    sha256,
    cloudModelName,
    instance,
    refresh,
  ]);

  const handleCancel = useCallback(
    async (composite: string) => {
      try {
        await cancelSession(composite);
        if (chatInstanceId === composite) {
          setChatInstanceId(null);
        }
      } catch (error) {
        setSpawnError(errorMessage(error));
      } finally {
        await refresh();
      }
    },
    [chatInstanceId, refresh],
  );

  const localSessions =
    sessions.status === "ready"
      ? sessions.sessions.filter((s) => s.provider === "local")
      : [];

  return {
    sessions,
    snapshot,
    provider,
    setProvider,
    runtimeBinding,
    setRuntimeBinding,
    artifactPath,
    setArtifactPath,
    sha256,
    setSha256,
    cloudModelName,
    setCloudModelName,
    instance,
    setInstance,
    spawning,
    spawnError,
    spawnNotice,
    chatInstanceId,
    setChatInstanceId,
    handleSpawn,
    handleCancel,
    localSessions,
  };
}

export type SwarmRoom = ReturnType<typeof useSwarmRoom>;

export function SwarmControlRoom() {
  const room = useSwarmRoom();

  return (
    <section
      className="swarm-control-room"
      data-testid="swarm-control-room"
      data-stable-id="swarm-control-room"
      aria-labelledby="swarm-control-room-title"
    >
      <header className="swarm-control-room__header">
        <h2 id="swarm-control-room-title">Swarm Control Room</h2>
        <p className="muted">
          Spin up local and cloud model sessions in parallel under hard bounds. Live
          sessions, resource budget, and the operator chat box all drive the real
          SwarmCoordinator backend.
        </p>
      </header>

      <SwarmResourceSection snapshot={room.snapshot} />
      <SwarmSpawnSection room={room} />
      <SwarmSessionsSection room={room} />

      <OperatorChat
        selectedInstanceId={room.chatInstanceId}
        localSessions={room.localSessions}
        onSelectInstance={room.setChatInstanceId}
      />
    </section>
  );
}

/** Resource budget bar block (wrapper handles loading/error states). */
export function SwarmResourceSection({ snapshot }: { snapshot: SnapshotState }) {
  return (
    <div
      className="swarm-resource-bar"
      data-stable-id="swarm-resource-bar"
      data-testid="swarm-resource-bar"
    >
      {snapshot.status === "loading" ? (
        <span className="muted">Loading resource snapshot...</span>
      ) : snapshot.status === "error" ? (
        <span className="swarm-error" data-testid="swarm-resource-error">
          Resource snapshot error: {snapshot.message}
        </span>
      ) : (
        <ResourceBar snapshot={snapshot.snapshot} />
      )}
    </div>
  );
}

/** Spawn-a-session form block. */
export function SwarmSpawnSection({ room }: { room: SwarmRoom }) {
  const {
    provider,
    setProvider,
    runtimeBinding,
    setRuntimeBinding,
    artifactPath,
    setArtifactPath,
    sha256,
    setSha256,
    cloudModelName,
    setCloudModelName,
    instance,
    setInstance,
    spawning,
    spawnError,
    spawnNotice,
    handleSpawn,
  } = room;

  return (
    <form
      className="swarm-spawn-form"
      data-stable-id="swarm-spawn-form"
      data-testid="swarm-spawn-form"
      onSubmit={(event) => {
        event.preventDefault();
        void handleSpawn();
      }}
    >
      <div className="swarm-spawn-form__row">
        <label>
          <span>Provider</span>
          <select
            value={provider}
            data-testid="swarm-spawn-provider"
            onChange={(event) => setProvider(event.target.value as SwarmProvider)}
          >
            <option value="local">Local (on-disk artifact)</option>
            <option value="byok_cloud">Cloud (BYOK)</option>
            <option value="official_cli">Cloud (official CLI)</option>
          </select>
        </label>
        <label>
          <span>Instance #</span>
          <input
            type="number"
            min={0}
            value={instance}
            data-testid="swarm-spawn-instance"
            onChange={(event) => setInstance(Math.max(0, Number(event.target.value) || 0))}
          />
        </label>
      </div>

      {provider === "local" ? (
        <>
          <div className="swarm-spawn-form__row">
            <label>
              <span>Runtime binding</span>
              <select
                value={runtimeBinding}
                data-testid="swarm-spawn-binding"
                onChange={(event) =>
                  setRuntimeBinding(event.target.value as SwarmRuntimeBinding)
                }
              >
                <option value="candle">candle (safetensors)</option>
                <option value="llama_cpp">llama.cpp (GGUF)</option>
              </select>
            </label>
          </div>
          <label className="swarm-spawn-form__full">
            <span>Model artifact path</span>
            <input
              type="text"
              value={artifactPath}
              placeholder="D:/models/tinyllama/model.safetensors"
              data-testid="swarm-spawn-artifact-path"
              onChange={(event) => setArtifactPath(event.target.value)}
            />
          </label>
          <label className="swarm-spawn-form__full">
            <span>Expected sha256 (integrity gate)</span>
            <input
              type="text"
              value={sha256}
              placeholder="64 hex chars"
              data-testid="swarm-spawn-sha256"
              onChange={(event) => setSha256(event.target.value)}
            />
          </label>
        </>
      ) : (
        <label className="swarm-spawn-form__full">
          <span>Cloud model name</span>
          <input
            type="text"
            value={cloudModelName}
            placeholder="gpt-4o / claude-sonnet-4"
            data-testid="swarm-spawn-cloud-model"
            onChange={(event) => setCloudModelName(event.target.value)}
          />
        </label>
      )}

      <div className="swarm-spawn-form__actions">
        <button type="submit" disabled={spawning} data-testid="swarm-spawn-submit">
          {spawning ? "Spawning..." : "Spawn session"}
        </button>
      </div>

      {spawnError ? (
        <p className="swarm-error" data-testid="swarm-spawn-error">
          {spawnError}
        </p>
      ) : null}
      {spawnNotice ? (
        <p className="swarm-notice" data-testid="swarm-spawn-notice">
          {spawnNotice}
        </p>
      ) : null}
    </form>
  );
}

/** Live sessions table block. */
export function SwarmSessionsSection({ room }: { room: SwarmRoom }) {
  const { sessions, setChatInstanceId, handleCancel } = room;
  return (
    <div
      className="swarm-sessions"
      data-stable-id="swarm-sessions"
      data-testid="swarm-sessions"
    >
      {sessions.status === "loading" ? (
        <p className="muted">Loading sessions...</p>
      ) : sessions.status === "error" ? (
        <p className="swarm-error" data-testid="swarm-sessions-error">
          {sessions.message}
        </p>
      ) : sessions.sessions.length === 0 ? (
        <p className="muted" data-testid="swarm-sessions-empty">
          No live sessions. Spawn one above.
        </p>
      ) : (
        <table className="swarm-sessions__table">
          <thead>
            <tr>
              <th>Model</th>
              <th>Inst</th>
              <th>State</th>
              <th>Provider</th>
              <th>Binding</th>
              <th>Source</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {sessions.sessions.map((session) => {
              const composite = session.instanceId.composite;
              return (
                <tr
                  key={composite}
                  data-testid={`swarm-session-row-${composite}`}
                  data-session-state={session.state}
                >
                  <td title={session.instanceId.modelId}>
                    <code>{session.instanceId.modelId.slice(0, 8)}</code>
                  </td>
                  <td>{session.instanceId.instance}</td>
                  <td>{session.state}</td>
                  <td>{session.provider}</td>
                  <td>{session.runtimeBinding}</td>
                  <td title={session.artifactPath ?? session.cloudModelName ?? ""}>
                    {session.artifactPath
                      ? session.artifactPath.split(/[/\\]/).pop()
                      : (session.cloudModelName ?? "—")}
                  </td>
                  <td>
                    {session.provider === "local" ? (
                      <button
                        type="button"
                        data-testid={`swarm-session-chat-${composite}`}
                        onClick={() => setChatInstanceId(composite)}
                      >
                        Chat
                      </button>
                    ) : null}
                    <button
                      type="button"
                      data-testid={`swarm-session-cancel-${composite}`}
                      onClick={() => void handleCancel(composite)}
                    >
                      Cancel
                    </button>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}
    </div>
  );
}

export function ResourceBar({ snapshot }: { snapshot: SwarmResourceSnapshot }) {
  const pct =
    snapshot.concurrencyCap > 0
      ? Math.round((snapshot.concurrencyInUse / snapshot.concurrencyCap) * 100)
      : 0;
  return (
    <div className="swarm-resource-bar__inner" data-testid="swarm-resource-bar-inner">
      <div className="swarm-resource-bar__meter" aria-label="Concurrency usage">
        <div className="swarm-resource-bar__track">
          <div
            className="swarm-resource-bar__fill"
            style={{ width: `${pct}%` }}
            data-testid="swarm-resource-bar-fill"
          />
        </div>
        <span>
          Concurrency: {snapshot.concurrencyInUse}/{snapshot.concurrencyCap} in use (
          {snapshot.concurrencyAvailable} free)
        </span>
      </div>
      <ul className="swarm-resource-bar__stats">
        <li data-testid="swarm-stat-live">Live sessions: {snapshot.liveSessions}</li>
        <li data-testid="swarm-stat-lifetime">
          Lifetime spawns remaining: {snapshot.lifetimeSpawnsRemaining}
        </li>
        <li data-testid="swarm-stat-tokens">
          Tokens remaining: {snapshot.tokensRemaining ?? "uncapped"}
        </li>
        <li data-testid="swarm-stat-cost">
          Cost remaining (micros): {snapshot.costMicrosRemaining ?? "uncapped"}
        </li>
        {snapshot.budgetExhausted ? (
          <li className="swarm-error" data-testid="swarm-stat-exhausted">
            Budget exhausted — spawns are blocked
          </li>
        ) : null}
      </ul>
    </div>
  );
}
