import { useState } from "react";

import { ApiKeyVault, type VaultLaneEntry } from "./ApiKeyVault";
import { ConsentPromptModal } from "./ConsentPromptModal";

// MT-129: top-level cloud-lane registration + management panel.
// Composes ApiKeyVault + ConsentPromptModal + lane registration
// table. Live IPC for vault read/write, lane registration, and
// consent prompt arrival is deferred to a follow-on; this surface
// renders the structural shell + accepts props from the parent
// (typically the future ModelRuntime control panel route).

export type CloudLaneKind = "openai_byok" | "anthropic_byok" | "official_cli";

export interface RegisteredCloudLane {
  laneId: string;
  kind: CloudLaneKind;
  displayName: string;
  modelName: string;
  hasSecret: boolean;
  enabled: boolean;
}

export interface PendingConsentPrompt {
  sessionId: string;
  lane: string;
  modelName: string;
  promptPreview?: string;
}

type Props = {
  lanes: RegisteredCloudLane[];
  vaultEntries: VaultLaneEntry[];
  pendingConsentPrompt?: PendingConsentPrompt;
  onRegisterLane: (
    kind: CloudLaneKind,
    laneId: string,
    modelName: string,
  ) => void | Promise<void>;
  onRemoveLane: (laneId: string) => void | Promise<void>;
  onToggleEnabled: (laneId: string, enabled: boolean) => void | Promise<void>;
  onPutSecret: (lane: string, secret: string) => void | Promise<void>;
  onDeleteSecret: (lane: string) => void | Promise<void>;
  onApproveConsent: () => void | Promise<void>;
  onDenyConsent: () => void | Promise<void>;
};

const LANE_KIND_LABELS: Record<CloudLaneKind, string> = {
  openai_byok: "OpenAI BYOK",
  anthropic_byok: "Anthropic BYOK",
  official_cli: "Official CLI",
};

const LANE_KINDS: CloudLaneKind[] = ["openai_byok", "anthropic_byok", "official_cli"];

export function CloudLanePanel({
  lanes,
  vaultEntries,
  pendingConsentPrompt,
  onRegisterLane,
  onRemoveLane,
  onToggleEnabled,
  onPutSecret,
  onDeleteSecret,
  onApproveConsent,
  onDenyConsent,
}: Props) {
  const [newKind, setNewKind] = useState<CloudLaneKind>("openai_byok");
  const [newLaneId, setNewLaneId] = useState("");
  const [newModelName, setNewModelName] = useState("");

  const canRegister =
    newLaneId.trim().length > 0 && newModelName.trim().length > 0;

  const handleRegister = async () => {
    if (!canRegister) return;
    await onRegisterLane(newKind, newLaneId.trim(), newModelName.trim());
    setNewLaneId("");
    setNewModelName("");
  };

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
                  <td>
                    <label className="cloud-lane-panel__toggle">
                      <input
                        type="checkbox"
                        checked={lane.enabled}
                        onChange={(event) =>
                          void onToggleEnabled(lane.laneId, event.target.checked)
                        }
                        data-testid={`cloud-lane-panel.lanes.row.${lane.laneId}.toggle`}
                      />
                      <span>{lane.enabled ? "on" : "off"}</span>
                    </label>
                  </td>
                  <td>
                    <button
                      type="button"
                      onClick={() => void onRemoveLane(lane.laneId)}
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
        lanes={vaultEntries}
        onPutSecret={onPutSecret}
        onDeleteSecret={onDeleteSecret}
      />

      <ConsentPromptModal
        open={Boolean(pendingConsentPrompt)}
        sessionId={pendingConsentPrompt?.sessionId ?? ""}
        lane={pendingConsentPrompt?.lane ?? ""}
        modelName={pendingConsentPrompt?.modelName ?? ""}
        promptPreview={pendingConsentPrompt?.promptPreview}
        onApprove={onApproveConsent}
        onDeny={onDenyConsent}
      />
    </section>
  );
}
