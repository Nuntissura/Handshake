import { useCallback, useEffect, useRef, useState } from "react";
import {
  cancelSession,
  listActiveSessions,
  listWorktrees,
  resourceSnapshot,
  spawnLocalCloudPair,
  spawnSession,
  spawnWithCloudEscalation,
  type ByokCloudProvider,
  type SessionSpawnTemplate,
  type SwarmIsolationTier,
  type SwarmLocalExecutionMode,
  type SwarmProvider,
  type SwarmResourceSnapshot,
  type SwarmRuntimeBinding,
  type SwarmSession,
  type SwarmSpawnRequest,
  type SwarmWorktree,
} from "../../lib/ipc/swarm_runtime";
import { OperatorChat, type OperatorChatCloudEscalation } from "./OperatorChat";

/** Sentinel option that reveals the free-text "new worktree" input. */
export const NEW_WORKTREE_SENTINEL = "__new__";

export type SwarmSpawnWorkflow = "single" | "local_cloud_pair" | "local_cloud_escalation";

/** Isolation tiers offered in the recorded-only selector (none + the 3 tiers). */
export const ISOLATION_TIER_OPTIONS: ReadonlyArray<{ value: SwarmIsolationTier; label: string }> = [
  { value: "tier1_container", label: "tier1_container (shared-kernel container)" },
  { value: "tier2_syscall", label: "tier2_syscall (syscall-filtered)" },
  { value: "tier3_microvm", label: "tier3_microvm (hardware-isolated microVM)" },
];

export const LOCAL_EXECUTION_MODE_OPTIONS: ReadonlyArray<{
  value: SwarmLocalExecutionMode;
  label: string;
}> = [
  { value: "cold", label: "Cold local" },
  { value: "warm_vm", label: "Warm VM" },
];

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

const BYTES_PER_MIB = 1024 * 1024;
const BYTES_PER_GIB = 1024 * BYTES_PER_MIB;

export function committedMemoryMiBToBytes(value: string): number | undefined {
  const trimmed = value.trim();
  if (!trimmed) return undefined;
  const parsed = Number(trimmed);
  if (!Number.isFinite(parsed) || parsed <= 0) return undefined;
  return Math.ceil(parsed * BYTES_PER_MIB);
}

function committedMemoryBytesToMiBValue(bytes?: number | null): string {
  if (!bytes || bytes <= 0) return "";
  return String(Math.ceil(bytes / BYTES_PER_MIB));
}

export function committedMemorySubmitBytes(
  value: string,
  preservedBytes?: number | null,
): number | undefined {
  const trimmed = value.trim();
  if (!trimmed) return undefined;
  if (
    preservedBytes &&
    preservedBytes > 0 &&
    committedMemoryBytesToMiBValue(preservedBytes) === trimmed
  ) {
    return preservedBytes;
  }
  return committedMemoryMiBToBytes(value);
}

function formatBudgetBytes(bytes?: number | null): string {
  if (bytes === undefined || bytes === null) return "uncapped";
  if (bytes >= BYTES_PER_GIB) return `${(bytes / BYTES_PER_GIB).toFixed(1)} GiB`;
  if (bytes >= BYTES_PER_MIB) return `${Math.round(bytes / BYTES_PER_MIB)} MiB`;
  return `${bytes} B`;
}

export function isLocalCommittedMemoryOnlyExhausted(
  snapshot: SwarmResourceSnapshot,
): boolean {
  return (
    snapshot.budgetExhausted &&
    snapshot.committedMemoryBytesRemaining === 0 &&
    snapshot.lifetimeSpawnsRemaining !== 0 &&
    snapshot.tokensRemaining !== 0 &&
    snapshot.costMicrosRemaining !== 0
  );
}

export function swarmBudgetExhaustionLabel(
  snapshot: SwarmResourceSnapshot,
): string | null {
  if (!snapshot.budgetExhausted) return null;
  if (isLocalCommittedMemoryOnlyExhausted(snapshot)) {
    return "Local committed memory exhausted - local spawns are blocked; cloud lanes remain available";
  }
  return "Budget exhausted - spawns are blocked";
}

export function swarmResourceBadge(snapshot: SwarmResourceSnapshot): string {
  if (isLocalCommittedMemoryOnlyExhausted(snapshot)) return "local memory exhausted";
  if (snapshot.budgetExhausted) return "budget exhausted";
  return `${snapshot.concurrencyInUse}/${snapshot.concurrencyCap} in use`;
}

/**
 * Resolve the worktree the operator actually assigned from the picker state:
 * when the "+ New worktree…" sentinel is selected, the free-text value wins;
 * otherwise the chosen existing id. Trimmed; blank => "" (unassigned, honest).
 */
export function effectiveWorktreeId(selection: string, newValue: string): string {
  const raw = selection === NEW_WORKTREE_SENTINEL ? newValue : selection;
  return raw.trim();
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
  const [spawnWorkflow, setSpawnWorkflow] = useState<SwarmSpawnWorkflow>("single");
  const [provider, setProvider] = useState<SwarmProvider>("local");
  const [cloudProvider, setCloudProvider] =
    useState<Extract<SwarmProvider, "byok_cloud" | "official_cli">>("byok_cloud");
  const [byokCloudProvider, setByokCloudProvider] = useState<ByokCloudProvider>("openai");
  const [runtimeBinding, setRuntimeBinding] = useState<SwarmRuntimeBinding>("candle");
  const [localExecutionMode, setLocalExecutionMode] =
    useState<SwarmLocalExecutionMode>("cold");
  const [artifactPath, setArtifactPath] = useState("");
  const [sha256, setSha256] = useState("");
  const [cloudModelName, setCloudModelName] = useState("");
  const [instance, setInstance] = useState(0);
  const [spawning, setSpawning] = useState(false);
  const [spawnError, setSpawnError] = useState<string | null>(null);
  const [spawnNotice, setSpawnNotice] = useState<string | null>(null);

  // Worktree / disk-location / isolation-tier ASSIGNMENT state (governance glue
  // #2: assign a session a place on disk or a VM/sandbox worktree). Tier3 local
  // llama.cpp is load-bearing when sandbox support is wired; other combinations
  // remain attribution.
  //
  // `worktreeSelection` is the <select> value: a discovered worktree id, the
  // NEW_WORKTREE_SENTINEL (reveal the free-text input), or "" (unassigned).
  const [worktrees, setWorktrees] = useState<SwarmWorktree[]>([]);
  const [swarmId, setSwarmId] = useState("");
  const [worktreeSelection, setWorktreeSelection] = useState("");
  const [newWorktreeId, setNewWorktreeId] = useState("");
  const [workingDir, setWorkingDir] = useState("");
  const [isolationTier, setIsolationTier] = useState<SwarmIsolationTier | "">("");
  const [committedMemoryMiB, setCommittedMemoryMiB] = useState("");
  const [committedMemoryPrefillBytes, setCommittedMemoryPrefillBytes] =
    useState<number | null>(null);

  // Selected session for the operator chat box (composite instance id).
  const [chatInstanceId, setChatInstanceId] = useState<string | null>(null);

  const pollRef = useRef<number | null>(null);

  const refreshWorktrees = useCallback(async () => {
    try {
      const rows = await listWorktrees();
      setWorktrees(rows);
    } catch {
      // Discovery is best-effort: a failure here must NOT block spawning, since
      // the form always also offers a free-text "new worktree" entry. Keep the
      // last good list (or empty) and stay silent.
    }
  }, []);

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
      await refreshWorktrees();
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
  }, [refresh, refreshWorktrees]);

  const handleSpawn = useCallback(async () => {
    setSpawnError(null);
    setSpawnNotice(null);
    setSpawning(true);
    try {
      const applyAssignment = (request: SwarmSpawnRequest) => {
        const wt = effectiveWorktreeId(worktreeSelection, newWorktreeId);
        const swarm = swarmId.trim() || (spawnWorkflow !== "single" ? wt : "");
        if (swarm) request.swarmId = swarm;
        if (wt) request.worktreeId = wt;
        const wd = workingDir.trim();
        if (wd) request.workingDir = wd;
        if (isolationTier) request.isolationTier = isolationTier;
        const committedBytes = committedMemorySubmitBytes(
          committedMemoryMiB,
          committedMemoryPrefillBytes,
        );
        if (request.provider === "local" && committedBytes) {
          request.committedMemoryBytes = committedBytes;
        }
        return request;
      };
      const localRequest = () =>
        applyAssignment({
          provider: "local",
          runtimeBinding,
          ...(localExecutionMode === "warm_vm"
            ? { localExecutionMode }
            : {}),
          artifactPath: artifactPath.trim(),
          sha256Expected: sha256.trim(),
          instance,
        });
      const cloudRequest = (selectedProvider: Extract<SwarmProvider, "byok_cloud" | "official_cli">) =>
        applyAssignment({
          provider: selectedProvider,
          cloudModelName: cloudModelName.trim(),
          ...(selectedProvider === "byok_cloud" ? { byokCloudProvider } : {}),
          instance,
        });

      let selectedComposite: string | null = null;
      if (spawnWorkflow === "local_cloud_pair") {
        const result = await spawnLocalCloudPair({
          local: localRequest(),
          cloud: cloudRequest(cloudProvider),
        });
        selectedComposite = result.local.instanceId?.composite ?? result.cloud.instanceId?.composite ?? null;
        const localText = result.local.instanceId
          ? `local ${result.local.instanceId.composite}`
          : `local failed: ${result.local.error}`;
        const cloudText = result.cloud.instanceId
          ? `cloud ${result.cloud.instanceId.composite}`
          : `cloud failed: ${result.cloud.error}`;
        setSpawnNotice(`Pair attempted: ${localText}; ${cloudText}`);
      } else if (spawnWorkflow === "local_cloud_escalation") {
        const result = await spawnWithCloudEscalation({
          local: localRequest(),
          cloud: cloudRequest(cloudProvider),
        });
        selectedComposite =
          result.cloud?.instanceId?.composite ?? result.local.instanceId?.composite ?? null;
        if (result.selected === "local" && result.local.instanceId) {
          setSpawnNotice(`Spawned local session ${result.local.instanceId.composite}`);
        } else if (result.selected === "cloud" && result.cloud?.instanceId) {
          setSpawnNotice(
            `Escalated to cloud session ${result.cloud.instanceId.composite}: ${result.escalationReason}`,
          );
        } else {
          setSpawnNotice(`No session spawned: ${result.local.error ?? "local spawn failed"}`);
        }
      } else {
        const request =
          provider === "local"
            ? localRequest()
            : cloudRequest(provider as Extract<SwarmProvider, "byok_cloud" | "official_cli">);
        const id = await spawnSession(request);
        selectedComposite = id.composite;
        setSpawnNotice(`Spawned session ${id.composite}`);
      }
      // Auto-select the new session for the chat box — ANY provider is chattable
      // now (the chat generate path is provider-agnostic), not just local.
      if (selectedComposite) setChatInstanceId(selectedComposite);
      await refresh();
      await refreshWorktrees();
    } catch (error) {
      // Surface the REAL backend error verbatim (e.g. PROVIDER_NOT_CONFIGURED,
      // FactoryFailed sha mismatch, missing artifact).
      setSpawnError(errorMessage(error));
    } finally {
      setSpawning(false);
    }
  }, [
    spawnWorkflow,
    provider,
    cloudProvider,
    byokCloudProvider,
    runtimeBinding,
    localExecutionMode,
    artifactPath,
    sha256,
    cloudModelName,
    instance,
    worktreeSelection,
    swarmId,
    newWorktreeId,
    workingDir,
    isolationTier,
    committedMemoryMiB,
    committedMemoryPrefillBytes,
    refresh,
    refreshWorktrees,
  ]);

  // ROI #3 STATE RECOVERY (edit-then-resume): prefill the spawn form from a
  // recorded session's stored template so the operator can tweak it (repoint a
  // moved artifact, change worktree) before re-spawning through the EXISTING
  // validated spawn path — no new spawn logic. Reuses the same form setters as
  // first-spawn; the operator then hits the normal Spawn button. We thread a
  // known worktree id through the free-text "new worktree" entry so the value is
  // always visible + editable even if it is not in the discovered list.
  const prefillSpawnForm = useCallback((tpl: SessionSpawnTemplate) => {
    setProvider(tpl.provider);
    setRuntimeBinding(tpl.runtimeBinding ?? "candle");
    setLocalExecutionMode(tpl.localExecutionMode ?? "cold");
    setArtifactPath(tpl.artifactPath ?? "");
    setSha256(tpl.sha256Expected ?? "");
    setCloudModelName(tpl.cloudModelName ?? "");
    setByokCloudProvider(tpl.byokCloudProvider ?? "openai");
    setSwarmId(tpl.swarmId ?? "");
    if (tpl.worktreeId) {
      setWorktreeSelection(NEW_WORKTREE_SENTINEL);
      setNewWorktreeId(tpl.worktreeId);
    } else {
      setWorktreeSelection("");
      setNewWorktreeId("");
    }
    setWorkingDir(tpl.workingDir ?? "");
    setIsolationTier(tpl.isolationTier ?? "");
    const templateMemoryBytes =
      tpl.provider === "local" ? (tpl.committedMemoryBytes ?? null) : null;
    setCommittedMemoryPrefillBytes(templateMemoryBytes);
    setCommittedMemoryMiB(committedMemoryBytesToMiBValue(templateMemoryBytes));
    // A resume mints a fresh instance ordinal; default to 0 (the operator can
    // bump it if they intend a concurrent peer of an existing instance).
    setInstance(0);
    setSpawnError(null);
    setSpawnNotice(`Prefilled from recorded session ${tpl.originSessionId} — edit and Spawn to resume`);
  }, []);

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

  // ALL spawned sessions feed the chat picker (governance glue #3): the chat
  // generate path is provider-agnostic, so cloud BYOK and official-CLI sessions
  // are chattable too — no provider filter. The picker labels each by provider
  // and disables non-live (not READY/GENERATING) options honestly.
  const allSessions =
    sessions.status === "ready" ? sessions.sessions : [];

  return {
    sessions,
    snapshot,
    spawnWorkflow,
    setSpawnWorkflow,
    provider,
    setProvider,
    cloudProvider,
    setCloudProvider,
    byokCloudProvider,
    setByokCloudProvider,
    runtimeBinding,
    setRuntimeBinding,
    localExecutionMode,
    setLocalExecutionMode,
    artifactPath,
    setArtifactPath,
    sha256,
    setSha256,
    cloudModelName,
    setCloudModelName,
    instance,
    setInstance,
    worktrees,
    swarmId,
    setSwarmId,
    worktreeSelection,
    setWorktreeSelection,
    newWorktreeId,
    setNewWorktreeId,
    workingDir,
    setWorkingDir,
    isolationTier,
    setIsolationTier,
    committedMemoryMiB,
    setCommittedMemoryMiB,
    spawning,
    spawnError,
    spawnNotice,
    chatInstanceId,
    setChatInstanceId,
    handleSpawn,
    handleCancel,
    prefillSpawnForm,
    refresh,
    allSessions,
  };
}

export type SwarmRoom = ReturnType<typeof useSwarmRoom>;

export function operatorChatCloudEscalation(room: SwarmRoom): OperatorChatCloudEscalation {
  const selectedSession = room.chatInstanceId
    ? room.allSessions.find((session) => session.instanceId.composite === room.chatInstanceId)
    : null;
  const worktreeSelection = room.worktreeSelection ?? "";
  const newWorktreeId = room.newWorktreeId ?? "";
  const cloudProvider = room.cloudProvider ?? "byok_cloud";
  const byokCloudProvider = room.byokCloudProvider ?? "openai";
  const worktreeId =
    selectedSession?.worktreeId
    ?? effectiveWorktreeId(worktreeSelection, newWorktreeId);
  const workingDir = selectedSession?.workingDir ?? (room.workingDir ?? "").trim();
  const cloudModelName = (room.cloudModelName ?? "").trim();
  const request: SwarmSpawnRequest = {
    provider: cloudProvider,
    cloudModelName,
    instance: room.instance ?? 0,
    ...(cloudProvider === "byok_cloud"
      ? { byokCloudProvider }
      : {}),
  };
  const swarmId = (selectedSession?.worktreeId ?? (room.swarmId ?? "").trim()) || worktreeId;
  if (swarmId) request.swarmId = swarmId;
  if (worktreeId) request.worktreeId = worktreeId;
  if (workingDir) request.workingDir = workingDir;
  const lane =
    cloudProvider === "byok_cloud" ? byokCloudProvider : "official_cli";
  return {
    request,
    label: `${lane} · ${cloudModelName || "cloud model not set"}`,
  };
}

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
        sessions={room.allSessions}
        onSelectInstance={room.setChatInstanceId}
        cloudEscalation={operatorChatCloudEscalation(room)}
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
    spawnWorkflow,
    setSpawnWorkflow,
    provider,
    setProvider,
    cloudProvider,
    setCloudProvider,
    byokCloudProvider,
    setByokCloudProvider,
    runtimeBinding,
    setRuntimeBinding,
    localExecutionMode,
    setLocalExecutionMode,
    artifactPath,
    setArtifactPath,
    sha256,
    setSha256,
    cloudModelName,
    setCloudModelName,
    instance,
    setInstance,
    worktrees,
    swarmId,
    setSwarmId,
    worktreeSelection,
    setWorktreeSelection,
    newWorktreeId,
    setNewWorktreeId,
    workingDir,
    setWorkingDir,
    isolationTier,
    setIsolationTier,
    committedMemoryMiB,
    setCommittedMemoryMiB,
    spawning,
    spawnError,
    spawnNotice,
    handleSpawn,
  } = room;

  const showNewWorktreeInput = worktreeSelection === NEW_WORKTREE_SENTINEL;
  const needsLocalConfig = spawnWorkflow !== "single" || provider === "local";
  const needsCloudConfig = spawnWorkflow !== "single" || provider !== "local";
  const selectedCloudProvider =
    spawnWorkflow === "single" ? provider : cloudProvider;
  const showByokFlavor = needsCloudConfig && selectedCloudProvider === "byok_cloud";
  const showCommittedMemoryInput = needsLocalConfig;

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
          <span>Workflow</span>
          <select
            value={spawnWorkflow}
            data-testid="swarm-spawn-workflow"
            onChange={(event) => setSpawnWorkflow(event.target.value as SwarmSpawnWorkflow)}
          >
            <option value="single">Single session</option>
            <option value="local_cloud_pair">Local + cloud pair</option>
            <option value="local_cloud_escalation">Local then cloud on capacity</option>
          </select>
        </label>
        {spawnWorkflow === "single" ? (
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
        ) : (
          <label>
            <span>Cloud peer</span>
            <select
              value={cloudProvider}
              data-testid="swarm-spawn-cloud-provider"
              onChange={(event) =>
                setCloudProvider(event.target.value as Extract<SwarmProvider, "byok_cloud" | "official_cli">)
              }
            >
              <option value="byok_cloud">Cloud (BYOK)</option>
              <option value="official_cli">Cloud (official CLI)</option>
            </select>
          </label>
        )}
      </div>

      <div className="swarm-spawn-form__row">
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

      {needsLocalConfig ? (
        <>
          {needsCloudConfig ? <h3 className="swarm-spawn-form__subhead">Local model</h3> : null}
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
            <label>
              <span>Execution mode</span>
              <select
                value={localExecutionMode}
                data-testid="swarm-spawn-local-execution-mode"
                onChange={(event) =>
                  setLocalExecutionMode(event.target.value as SwarmLocalExecutionMode)
                }
              >
                {LOCAL_EXECUTION_MODE_OPTIONS.map((opt) => (
                  <option key={opt.value} value={opt.value}>
                    {opt.label}
                  </option>
                ))}
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
      ) : null}

      {needsCloudConfig ? (
        <>
          <label className="swarm-spawn-form__full">
            <span>{needsLocalConfig ? "Cloud peer model name" : "Cloud model name"}</span>
            <input
              type="text"
              value={cloudModelName}
              placeholder="gpt-4o / claude-sonnet-4"
              data-testid="swarm-spawn-cloud-model"
              onChange={(event) => setCloudModelName(event.target.value)}
            />
          </label>
          {showByokFlavor ? (
            <label className="swarm-spawn-form__full">
              <span>BYOK lane</span>
              <select
                value={byokCloudProvider}
                data-testid="swarm-spawn-byok-provider"
                onChange={(event) => setByokCloudProvider(event.target.value as ByokCloudProvider)}
              >
                <option value="openai">OpenAI</option>
                <option value="anthropic">Anthropic</option>
              </select>
            </label>
          ) : null}
        </>
      ) : null}

      {/* Worktree / disk-location / isolation-tier ASSIGNMENT (governance glue
          #2). Provider-independent — local AND cloud sessions can be assigned. */}
      <fieldset
        className="swarm-spawn-form__assignment"
        data-testid="swarm-spawn-assignment"
      >
        <legend>Assignment (optional)</legend>

        <label className="swarm-spawn-form__full">
          <span>Swarm id</span>
          <input
            type="text"
            value={swarmId}
            placeholder="e.g. research-swarm (pair/escalation uses worktree if blank)"
            data-testid="swarm-spawn-swarm-id"
            onChange={(event) => setSwarmId(event.target.value)}
          />
        </label>

        <div className="swarm-spawn-form__row">
          <label>
            <span>VM / sandbox worktree</span>
            <select
              value={worktreeSelection}
              data-testid="swarm-spawn-worktree-select"
              onChange={(event) => setWorktreeSelection(event.target.value)}
            >
              <option value="">— Unassigned —</option>
              {worktrees.map((wt) => (
                <option key={wt.worktreeId} value={wt.worktreeId}>
                  {wt.worktreeId} ({wt.liveSessionCount} live)
                </option>
              ))}
              <option value={NEW_WORKTREE_SENTINEL}>+ New worktree…</option>
            </select>
          </label>
        </div>

        {showNewWorktreeInput ? (
          <label className="swarm-spawn-form__full">
            <span>New worktree name</span>
            <input
              type="text"
              value={newWorktreeId}
              placeholder="e.g. wt-feature-x"
              data-testid="swarm-spawn-worktree-new"
              onChange={(event) => setNewWorktreeId(event.target.value)}
            />
          </label>
        ) : null}

        <label className="swarm-spawn-form__full">
          <span>Working dir (on disk)</span>
          <input
            type="text"
            value={workingDir}
            placeholder="e.g. D:/work/wt-foo or ./worktrees/foo (optional)"
            data-testid="swarm-spawn-working-dir"
            onChange={(event) => setWorkingDir(event.target.value)}
          />
        </label>

        <div className="swarm-spawn-form__row">
          <label>
            <span>Isolation tier</span>
            <select
              value={isolationTier}
              data-testid="swarm-spawn-isolation-tier"
              onChange={(event) =>
                setIsolationTier(event.target.value as SwarmIsolationTier | "")
              }
            >
              <option value="">— None —</option>
              {ISOLATION_TIER_OPTIONS.map((opt) => (
                <option key={opt.value} value={opt.value}>
                  {opt.label}
                </option>
              ))}
            </select>
          </label>
          {showCommittedMemoryInput ? (
            <label>
              <span>Local committed memory (MiB)</span>
              <input
                type="number"
                min={1}
                step={1}
                value={committedMemoryMiB}
                placeholder="e.g. 6144"
                data-testid="swarm-spawn-committed-memory-mib"
                onChange={(event) => setCommittedMemoryMiB(event.target.value)}
              />
            </label>
          ) : null}
        </div>
        <p className="swarm-notice" data-testid="swarm-isolation-note">
          Tier3 local llama.cpp uses cold microVM unless Warm VM is selected.
          Warm VM requires an assigned worktree and resident guest agent support.
          Committed memory is reserved when the app run has a memory ceiling.
        </p>
      </fieldset>

      <div className="swarm-spawn-form__actions">
        <button type="submit" disabled={spawning} data-testid="swarm-spawn-submit">
          {spawning
            ? "Spawning..."
            : spawnWorkflow === "local_cloud_pair"
              ? "Spawn pair"
              : spawnWorkflow === "local_cloud_escalation"
                ? "Spawn with escalation"
                : "Spawn session"}
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
              <th>Mode</th>
              <th>Worktree</th>
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
                  <td data-testid={`swarm-session-execution-mode-${composite}`}>
                    {session.localExecutionMode ?? "—"}
                  </td>
                  <td
                    data-testid={`swarm-session-worktree-${composite}`}
                    title={session.workingDir ?? undefined}
                  >
                    {session.worktreeId ?? "—"}
                  </td>
                  <td title={session.artifactPath ?? session.cloudModelName ?? ""}>
                    {session.artifactPath
                      ? session.artifactPath.split(/[/\\]/).pop()
                      : (session.cloudModelName ?? "—")}
                  </td>
                  <td>
                    {/* Every provider is chattable now (governance glue #3): the
                        generate path is provider-agnostic, so local, cloud, and
                        CLI rows all offer Chat. */}
                    <button
                      type="button"
                      data-testid={`swarm-session-chat-${composite}`}
                      onClick={() => setChatInstanceId(composite)}
                    >
                      Chat
                    </button>
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
  const exhaustionLabel = swarmBudgetExhaustionLabel(snapshot);
  const committedCap = snapshot.committedMemoryBytesCap ?? null;
  const committedRemaining = snapshot.committedMemoryBytesRemaining ?? null;
  const committedMemoryLabel =
    committedCap === null
      ? "uncapped"
      : `${formatBudgetBytes(committedRemaining)} / ${formatBudgetBytes(committedCap)} remaining`;
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
        <li data-testid="swarm-stat-committed-memory">
          Committed memory: {committedMemoryLabel}
        </li>
        {exhaustionLabel ? (
          <li className="swarm-error" data-testid="swarm-stat-exhausted">
            {exhaustionLabel}
          </li>
        ) : null}
      </ul>
    </div>
  );
}
