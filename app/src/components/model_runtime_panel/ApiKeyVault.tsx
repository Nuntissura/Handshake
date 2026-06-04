import { useState } from "react";

import { formatStoredOnDate } from "../../lib/ipc/cloud_lane";

// MT-129: API key vault UI for the ModelRuntime control panel.
// Surface displays + edits the OS-keychain-backed SecretsVault
// (MT-128). Per HBR-INT-005 the input is masked at-rest in the DOM
// (type=password); deletion requires explicit confirmation; the
// vault list shows lane id + "stored on YYYY-MM-DD" indicator
// (NEVER the secret value). All save / delete actions invoke
// real Tauri commands round-tripping through the OS keychain.

export interface VaultLaneEntry {
  lane: string;
  hasSecret: boolean;
  updatedAtUtc?: string;
}

type Props = {
  lanes: VaultLaneEntry[];
  onPutSecret: (lane: string, secret: string) => void | Promise<void>;
  onDeleteSecret: (lane: string) => void | Promise<void>;
};

export function ApiKeyVault({ lanes, onPutSecret, onDeleteSecret }: Props) {
  const [editingLane, setEditingLane] = useState<string | null>(null);
  const [draftSecret, setDraftSecret] = useState("");
  const [confirmingDelete, setConfirmingDelete] = useState<string | null>(null);

  const handleSave = async () => {
    if (!editingLane) return;
    if (draftSecret.trim().length === 0) return;
    // GLOBAL: never log the draftSecret. Pass it through to the
    // parent handler and clear local state immediately.
    await onPutSecret(editingLane, draftSecret);
    setDraftSecret("");
    setEditingLane(null);
  };

  return (
    <section
      className="model-runtime-panel__panel api-key-vault"
      data-testid="api-key-vault"
      aria-labelledby="api-key-vault-title"
    >
      <header className="model-runtime-panel__panel-header">
        <h3 id="api-key-vault-title">API Key Vault</h3>
        <p className="muted" data-testid="api-key-vault.note">
          Operator-managed secret store. Keys are kept in the OS keychain (per
          MT-128 SecretsVault). The vault list shows lane + key-present
          indicator, NEVER the key value itself. Each cloud lane uses one
          secret.
        </p>
      </header>

      {lanes.length === 0 ? (
        <p className="muted" data-testid="api-key-vault.empty">
          No vault entries. Register a cloud lane to start.
        </p>
      ) : (
        <table data-testid="api-key-vault.table">
          <thead>
            <tr>
              <th>Lane</th>
              <th>Secret present</th>
              <th>Updated</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {lanes.map((entry) => (
              <tr
                key={entry.lane}
                data-testid={`api-key-vault.row.${entry.lane}`}
              >
                <td>
                  <code>{entry.lane}</code>
                </td>
                <td>
                  <span
                    className={
                      entry.hasSecret
                        ? "api-key-vault__status api-key-vault__status--present"
                        : "api-key-vault__status api-key-vault__status--missing"
                    }
                    data-testid={`api-key-vault.row.${entry.lane}.status`}
                  >
                    {entry.hasSecret ? "stored" : "missing"}
                  </span>
                </td>
                <td data-testid={`api-key-vault.row.${entry.lane}.stored-on`}>
                  {entry.hasSecret
                    ? `stored on ${formatStoredOnDate(entry.updatedAtUtc ?? null)}`
                    : "—"}
                </td>
                <td>
                  <button
                    type="button"
                    onClick={() => {
                      setEditingLane(entry.lane);
                      setDraftSecret("");
                    }}
                    data-testid={`api-key-vault.row.${entry.lane}.edit`}
                  >
                    Update key
                  </button>
                  {entry.hasSecret ? (
                    <button
                      type="button"
                      onClick={() => setConfirmingDelete(entry.lane)}
                      data-testid={`api-key-vault.row.${entry.lane}.delete`}
                    >
                      Delete
                    </button>
                  ) : null}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      {editingLane ? (
        <div
          className="api-key-vault__edit"
          data-testid="api-key-vault.edit-form"
        >
          <h4>Update key for {editingLane}</h4>
          <label>
            <span>New secret</span>
            <input
              type="password"
              value={draftSecret}
              onChange={(event) => setDraftSecret(event.target.value)}
              data-testid="api-key-vault.edit-form.secret"
              autoComplete="off"
            />
          </label>
          <div className="api-key-vault__edit-actions">
            <button
              type="button"
              onClick={() => void handleSave()}
              disabled={draftSecret.trim().length === 0}
              data-testid="api-key-vault.edit-form.save"
            >
              Save
            </button>
            <button
              type="button"
              onClick={() => {
                setDraftSecret("");
                setEditingLane(null);
              }}
              data-testid="api-key-vault.edit-form.cancel"
            >
              Cancel
            </button>
          </div>
        </div>
      ) : null}

      {confirmingDelete ? (
        <div
          className="api-key-vault__confirm-delete"
          data-testid="api-key-vault.confirm-delete"
          role="alert"
        >
          <p>
            Delete the stored secret for <code>{confirmingDelete}</code>? This
            cannot be undone.
          </p>
          <button
            type="button"
            onClick={() => {
              void onDeleteSecret(confirmingDelete);
              setConfirmingDelete(null);
            }}
            data-testid="api-key-vault.confirm-delete.confirm"
          >
            Delete
          </button>
          <button
            type="button"
            onClick={() => setConfirmingDelete(null)}
            data-testid="api-key-vault.confirm-delete.cancel"
          >
            Keep
          </button>
        </div>
      ) : null}
    </section>
  );
}
