import { useCallback, useEffect, useMemo, useState } from "react";

import {
  type CloudLaneKind,
  type CloudLaneSummary,
  type KeyMetadata,
  deleteApiKey,
  denyConsent,
  formatStoredOnDate,
  grantConsent,
  listCloudLanes,
  listStoredKeys,
  registerCloudLane,
  removeCloudLane,
  rotateApiKey,
  storeApiKey,
  toggleCloudLane,
} from "../../lib/ipc/cloud_lane";
import { ApiKeyVault, type VaultLaneEntry } from "./ApiKeyVault";
import { ConsentPromptModal } from "./ConsentPromptModal";

// MT-129: top-level cloud-lane registration + management panel.
// Composes ApiKeyVault + ConsentPromptModal + lane registration
// table. ALL data flows through real Tauri IPC commands; the
// backend round-trips through `OsKeychainSecretsVault` (Windows
// Credential Manager via the `keyring` crate) and `ConsentGate`.

export type { CloudLaneKind, CloudLaneSummary } from "../../lib/ipc/cloud_lane";

/** Re-exported for compatibility with the legacy `RegisteredCloudLane`
 * shape used by tests / call sites that pre-date the IPC refactor. */
export type RegisteredCloudLane = CloudLaneSummary;

export interface PendingConsentPrompt {
  sessionId: string;
  lane: string;
  modelName: string;
  promptPreview?: string;
}

type Props = {
  /** Optional consent prompt the parent forces open. If absent, the
   * panel itself manages the modal lifecycle for grant/deny calls
   * triggered by other UI surfaces or by direct operator action.
   * The Tauri side caches the (session, lane) decision in
   * `ConsentGate` regardless of which surface captured it. */
  pendingConsentPrompt?: PendingConsentPrompt;
  /** Called after the operator grants consent so the parent can
   * resume the gated cloud send. */
  onConsentResolved?: (decision: "approved" | "denied", lane: string) => void;
};

const LANE_KIND_LABELS: Record<CloudLaneKind, string> = {
  openai_byok: "OpenAI BYOK",
  anthropic_byok: "Anthropic BYOK",
  official_cli: "Official CLI",
};

const LANE_KINDS: CloudLaneKind[] = ["openai_byok", "anthropic_byok", "official_cli"];

/** Operator signature derived from the local browser env. The
 * production wiring will replace this with a real operator-identity
 * source once the session-identity surface lands; meanwhile every
 * IPC write carries a non-empty signature so the backend validation
 * succeeds. */
function operatorSignature(): string {
  const now = new Date();
  const ts = now.toISOString().replace(/[-:T.Z]/g, "").slice(0, 14);
  return `operator-${ts}`;
}

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  return "Unknown error";
}

export function CloudLanePanel({
  pendingConsentPrompt: externalPendingConsentPrompt,
  onConsentResolved,
}: Props = {}) {
  const [lanes, setLanes] = useState<CloudLaneSummary[]>([]);
  const [vaultEntries, setVaultEntries] = useState<KeyMetadata[]>([]);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [lastReceipt, setLastReceipt] = useState<string | null>(null);

  const [newKind, setNewKind] = useState<CloudLaneKind>("openai_byok");
  const [newLaneId, setNewLaneId] = useState("");
  const [newModelName, setNewModelName] = useState("");

  const [internalConsentPrompt, setInternalConsentPrompt] =
    useState<PendingConsentPrompt | null>(null);

  const activeConsentPrompt = externalPendingConsentPrompt ?? internalConsentPrompt;

  const refresh = useCallback(async () => {
    try {
      const [nextLanes, nextKeys] = await Promise.all([
        listCloudLanes(),
        listStoredKeys(),
      ]);
      setLanes(nextLanes);
      setVaultEntries(nextKeys);
      setLoadError(null);
    } catch (error) {
      setLoadError(errorMessage(error));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const canRegister =
    newLaneId.trim().length > 0 && newModelName.trim().length > 0;

  const handleRegister = async () => {
    if (!canRegister) return;
    try {
      await registerCloudLane({
        kind: newKind,
        laneId: newLaneId.trim(),
        modelName: newModelName.trim(),
      });
      setNewLaneId("");
      setNewModelName("");
      setLastReceipt(`Registered lane ${newLaneId.trim()}`);
      await refresh();
    } catch (error) {
      setLoadError(errorMessage(error));
    }
  };

  const handleRemoveLane = async (laneId: string) => {
    try {
      await removeCloudLane(laneId, operatorSignature());
      setLastReceipt(`Removed lane ${laneId}`);
      await refresh();
    } catch (error) {
      setLoadError(errorMessage(error));
    }
  };

  const handleToggleEnabled = async (laneId: string, enabled: boolean) => {
    try {
      await toggleCloudLane(laneId, enabled, operatorSignature());
      await refresh();
    } catch (error) {
      setLoadError(errorMessage(error));
    }
  };

  const handlePutSecret = async (lane: string, secret: string) => {
    try {
      // Pick rotate vs. store based on whether the lane already has
      // a stored secret: both code paths overwrite the keychain entry,
      // but the receipt action string differs for operator-facing
      // logging.
      const existing = vaultEntries.find((v) => v.lane === lane);
      const receipt = existing && existing.hasSecret
        ? await rotateApiKey({
            lane,
            secret,
            operatorSignature: operatorSignature(),
          })
        : await storeApiKey({
            lane,
            secret,
            operatorSignature: operatorSignature(),
          });
      setLastReceipt(
        `${receipt.action === "rotate" ? "Rotated" : "Stored"} key for ${lane}`,
      );
      await refresh();
    } catch (error) {
      setLoadError(errorMessage(error));
    }
  };

  const handleDeleteSecret = async (lane: string) => {
    try {
      await deleteApiKey(lane, operatorSignature());
      setLastReceipt(`Deleted key for ${lane}`);
      await refresh();
    } catch (error) {
      setLoadError(errorMessage(error));
    }
  };

  const handleApproveConsent = async () => {
    if (!activeConsentPrompt) return;
    try {
      await grantConsent(activeConsentPrompt.sessionId, activeConsentPrompt.lane);
      setLastReceipt(`Approved cloud send on ${activeConsentPrompt.lane}`);
      onConsentResolved?.("approved", activeConsentPrompt.lane);
    } catch (error) {
      setLoadError(errorMessage(error));
    } finally {
      setInternalConsentPrompt(null);
    }
  };

  const handleDenyConsent = async () => {
    if (!activeConsentPrompt) return;
    try {
      await denyConsent(activeConsentPrompt.sessionId, activeConsentPrompt.lane);
      setLastReceipt(`Denied cloud send on ${activeConsentPrompt.lane}`);
      onConsentResolved?.("denied", activeConsentPrompt.lane);
    } catch (error) {
      setLoadError(errorMessage(error));
    } finally {
      setInternalConsentPrompt(null);
    }
  };

  // Project the vault key metadata into the legacy `VaultLaneEntry`
  // shape the `ApiKeyVault` component expects.
  const apiKeyVaultEntries = useMemo<VaultLaneEntry[]>(() => {
    return vaultEntries.map((meta) => ({
      lane: meta.lane,
      hasSecret: meta.hasSecret,
      updatedAtUtc: meta.updatedAtUtc ?? undefined,
    }));
  }, [vaultEntries]);

  return (
    <section
      className="model-runtime-panel cloud-lane-panel"
      data-testid="cloud-lane-panel"
      aria-labelledby="cloud-lane-panel-title"
    >
      <header className="model-runtime-panel__header">
        <h2 id="cloud-lane-panel-title">Cloud Lanes</h2>
        <p className="muted" data-testid="cloud-lane-panel.note">
          Register BYOK OpenAI / Anthropic / Official-CLI bridge lanes
          (MT-125 / MT-126 / MT-127). Each lane uses one SecretsVault key
          (MT-128) and is gated by per-session ConsentGate.
        </p>
        {loadError ? (
          <p
            className="model-runtime-panel__error"
            data-testid="cloud-lane-panel.error"
            role="alert"
          >
            {loadError}
          </p>
        ) : null}
        {lastReceipt ? (
          <p
            className="model-runtime-panel__receipt"
            data-testid="cloud-lane-panel.receipt"
          >
            {lastReceipt}
          </p>
        ) : null}
      </header>

      <section
        className="cloud-lane-panel__register"
        data-testid="cloud-lane-panel.register"
      >
        <h3>Register a new lane</h3>
        <label>
          <span>Lane kind</span>
          <select
            value={newKind}
            onChange={(event) => setNewKind(event.target.value as CloudLaneKind)}
            data-testid="cloud-lane-panel.register.kind"
          >
            {LANE_KINDS.map((kind) => (
              <option key={kind} value={kind}>
                {LANE_KIND_LABELS[kind]}
              </option>
            ))}
          </select>
        </label>
        <label>
          <span>Lane id (operator name; used as vault key)</span>
          <input
            type="text"
            value={newLaneId}
            onChange={(event) => setNewLaneId(event.target.value)}
            data-testid="cloud-lane-panel.register.lane-id"
          />
        </label>
        <label>
          <span>Model name</span>
          <input
            type="text"
            value={newModelName}
            onChange={(event) => setNewModelName(event.target.value)}
            data-testid="cloud-lane-panel.register.model-name"
          />
        </label>
        <button
          type="button"
          onClick={() => void handleRegister()}
          disabled={!canRegister}
          data-testid="cloud-lane-panel.register.submit"
        >
          Register lane
        </button>
      </section>

      <section
        className="cloud-lane-panel__lanes"
        data-testid="cloud-lane-panel.lanes"
      >
        <h3>Registered lanes</h3>
        {lanes.length === 0 ? (
          <p className="muted" data-testid="cloud-lane-panel.lanes.empty">
            No cloud lanes registered.
          </p>
        ) : (
          <table data-testid="cloud-lane-panel.lanes.table">
            <thead>
              <tr>
                <th>Lane</th>
                <th>Kind</th>
                <th>Display</th>
                <th>Model</th>
                <th>Vault</th>
                <th>Stored on</th>
                <th>Enabled</th>
                <th>Actions</th>
              </tr>
            </thead>
            <tbody>
              {lanes.map((lane) => (
                <tr
                  key={lane.laneId}
                  data-testid={`cloud-lane-panel.lanes.row.${lane.laneId}`}
                >
                  <td>
                    <code>{lane.laneId}</code>
                  </td>
                  <td>{LANE_KIND_LABELS[lane.kind]}</td>
                  <td>{lane.displayName}</td>
                  <td>{lane.modelName}</td>
                  <td>
                    <span
                      data-testid={`cloud-lane-panel.lanes.row.${lane.laneId}.vault-status`}
                    >
                      {lane.hasSecret ? "stored" : "missing"}
                    </span>
                  </td>
                  <td
                    data-testid={`cloud-lane-panel.lanes.row.${lane.laneId}.stored-on`}
                  >
                    {formatStoredOnDate(lane.secretUpdatedAtUtc)}
                  </td>
                  <td>
                    <label className="cloud-lane-panel__toggle">
                      <input
                        type="checkbox"
                        checked={lane.enabled}
                        onChange={(event) =>
                          void handleToggleEnabled(lane.laneId, event.target.checked)
                        }
                        data-testid={`cloud-lane-panel.lanes.row.${lane.laneId}.toggle`}
                      />
                      <span>{lane.enabled ? "on" : "off"}</span>
                    </label>
                  </td>
                  <td>
                    <button
                      type="button"
                      onClick={() => void handleRemoveLane(lane.laneId)}
                      data-testid={`cloud-lane-panel.lanes.row.${lane.laneId}.remove`}
                    >
                      Remove
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      <ApiKeyVault
        lanes={apiKeyVaultEntries}
        onPutSecret={handlePutSecret}
        onDeleteSecret={handleDeleteSecret}
      />

      <ConsentPromptModal
        open={Boolean(activeConsentPrompt)}
        sessionId={activeConsentPrompt?.sessionId ?? ""}
        lane={activeConsentPrompt?.lane ?? ""}
        modelName={activeConsentPrompt?.modelName ?? ""}
        promptPreview={activeConsentPrompt?.promptPreview}
        onApprove={handleApproveConsent}
        onDeny={handleDenyConsent}
      />
    </section>
  );
}
