import { useEffect, useMemo, useState } from "react";
import {
  applyAtelierPatchsets,
  createJob,
  getAtelierRoles,
  getJob,
  sha256HexUtf8,
  type AtelierApplySuggestionV1,
  type AtelierRoleSummary,
  type Block,
  type DocPatchsetV1,
  type SelectionRangeV1,
} from "../lib/api";
import type { TiptapSelectionInfo } from "./TiptapEditor";

type RoleSuggestionV1 = {
  suggestion_id: string;
  role_id: string;
  title: string;
  rationale?: string | null;
  patchset: DocPatchsetV1;
  protocol_id: string;
  source_job_id?: string;
  source_trace_id?: string;
  source_model_id?: string;
};

type RoleSuggestionsResponseV1 = {
  schema_version: "hsk.atelier.role_suggestions@v1";
  doc_id: string;
  selection: SelectionRangeV1;
  by_role: Array<{ role_id: string; suggestions: RoleSuggestionV1[] }>;
};

type Props = {
  open: boolean;
  onClose: () => void;
  docId: string;
  docText: string;
  selection: TiptapSelectionInfo | null;
  disabledReason?: string | null;
  onAppliedBlocks: (blocks: Block[]) => void;
};

function sleep(ms: number) {
  return new Promise<void>((resolve) => {
    window.setTimeout(() => resolve(), ms);
  });
}

function isRoleSuggestionsResponseV1(value: unknown): value is RoleSuggestionsResponseV1 {
  if (!value || typeof value !== "object") return false;
  const obj = value as Record<string, unknown>;
  return obj.schema_version === "hsk.atelier.role_suggestions@v1" && Array.isArray(obj.by_role);
}

export function AtelierCollaborationPanel({
  open,
  onClose,
  docId,
  docText,
  selection,
  disabledReason,
  onAppliedBlocks,
}: Props) {
  const [roles, setRoles] = useState<AtelierRoleSummary[]>([]);
  const [rolesLoading, setRolesLoading] = useState(false);
  const [rolesError, setRolesError] = useState<string | null>(null);

  const [docPreimageHash, setDocPreimageHash] = useState<string | null>(null);
  const [docHashError, setDocHashError] = useState<string | null>(null);

  const [suggestionsByRole, setSuggestionsByRole] = useState<Record<string, RoleSuggestionV1[]>>(
    {},
  );
  const [roleStatusById, setRoleStatusById] = useState<Record<string, string | undefined>>({});
  const [selectedIds, setSelectedIds] = useState<Record<string, boolean>>({});
  const [applyError, setApplyError] = useState<string | null>(null);
  const [applying, setApplying] = useState(false);

  const selectionPreview = selection?.text.trim() ?? "";

  useEffect(() => {
    if (!open) return;
    setRolesLoading(true);
    setRolesError(null);
    setSuggestionsByRole({});
    setRoleStatusById({});
    setSelectedIds({});
    setApplyError(null);

    void getAtelierRoles()
      .then((response) => setRoles(response.roles))
      .catch((err) => {
        const message = err instanceof Error ? err.message : String(err);
        setRolesError(message);
      })
      .finally(() => setRolesLoading(false));
  }, [open]);

  useEffect(() => {
    if (!open) return;
    setDocPreimageHash(null);
    setDocHashError(null);
    void sha256HexUtf8(docText)
      .then((hash) => setDocPreimageHash(hash))
      .catch((err) => {
        const message = err instanceof Error ? err.message : String(err);
        setDocHashError(message);
      });
  }, [open, docText]);

  const canOperate = useMemo(() => {
    if (!open) return false;
    if (disabledReason) return false;
    if (!selection || selection.text.trim().length === 0) return false;
    if (!docPreimageHash) return false;
    return true;
  }, [disabledReason, docPreimageHash, open, selection]);

  const buildSelectionRange = async (): Promise<SelectionRangeV1> => {
    if (!selection || !docPreimageHash) throw new Error("Missing selection or doc hash");
    const selectionHash = await sha256HexUtf8(selection.text);
    return {
      schema_version: "hsk.selection_range@v1",
      surface: "docs",
      coordinate_space: "doc_text_utf8_v1",
      start_utf8: selection.startUtf8,
      end_utf8: selection.endUtf8,
      doc_preimage_sha256: docPreimageHash,
      selection_preimage_sha256: selectionHash,
    };
  };

  const generateForRole = async (roleId: string) => {
    if (!canOperate) return;
    setRoleStatusById((prev) => ({ ...prev, [roleId]: "Starting..." }));

    try {
      const selectionRange = await buildSelectionRange();
      const run = await createJob("doc_edit", "atelier-doc-suggest-v1", docId, {
        doc_id: docId,
        role_id: roleId,
        selection: selectionRange,
        max_suggestions: 3,
      });

      setRoleStatusById((prev) => ({ ...prev, [roleId]: `Running (job_id=${run.job_id})...` }));

      for (let attempt = 0; attempt < 60; attempt += 1) {
        const job = await getJob(run.job_id);
        if (job.state === "completed") {
          const out = job.job_outputs;
          if (!isRoleSuggestionsResponseV1(out)) {
            throw new Error("Unexpected job_outputs shape for doc_edit");
          }
          const entry = out.by_role.find((r) => r.role_id === roleId);
          const suggestions = entry?.suggestions ?? [];
          setSuggestionsByRole((prev) => ({ ...prev, [roleId]: suggestions }));
          setRoleStatusById((prev) => ({ ...prev, [roleId]: undefined }));
          return;
        }
        if (job.state === "failed" || job.state === "poisoned") {
          throw new Error(job.error_message ?? `Job ended in state: ${job.state}`);
        }
        await sleep(750);
      }

      throw new Error("Timed out waiting for doc_edit job to complete");
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setRoleStatusById((prev) => ({ ...prev, [roleId]: message }));
    }
  };

  const allSuggestions = useMemo(() => {
    const all: RoleSuggestionV1[] = [];
    Object.values(suggestionsByRole).forEach((items) => items.forEach((s) => all.push(s)));
    return all;
  }, [suggestionsByRole]);

  const applySelected = async () => {
    setApplyError(null);
    const selectedSuggestions: AtelierApplySuggestionV1[] = [];

    for (const suggestion of allSuggestions) {
      if (!selectedIds[suggestion.suggestion_id]) continue;
      selectedSuggestions.push({
        role_id: suggestion.role_id,
        suggestion_id: suggestion.suggestion_id,
        patchset: suggestion.patchset,
      });
    }

    if (selectedSuggestions.length === 0) {
      setApplyError("Select at least one suggestion to apply.");
      return;
    }

    setApplying(true);
    try {
      const selection = selectedSuggestions[0]!.patchset.selection;
      const updatedBlocks = await applyAtelierPatchsets(docId, {
        doc_id: docId,
        selection,
        suggestions_to_apply: selectedSuggestions,
      });
      onAppliedBlocks(updatedBlocks);
      onClose();
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setApplyError(message);
    } finally {
      setApplying(false);
    }
  };

  if (!open) return null;

  return (
    <aside className="atelier-collab-panel" aria-label="Atelier collaboration panel">
      <div className="atelier-collab-panel__header">
        <h3 className="atelier-collab-panel__title">Atelier Collaboration</h3>
        <button type="button" className="atelier-collab-panel__close" onClick={onClose} aria-label="Close">
          ×
        </button>
      </div>

      <section className="atelier-collab-panel__section">
        <h4 className="atelier-collab-panel__section-title">Selection</h4>
        {selectionPreview.length === 0 ? (
          <p className="muted">{disabledReason ?? "No selection."}</p>
        ) : (
          <pre className="atelier-collab-panel__selection">{selectionPreview}</pre>
        )}
        {docHashError && <p className="error">Error: {docHashError}</p>}
      </section>

      <section className="atelier-collab-panel__section">
        <div className="atelier-collab-panel__section-header">
          <h4 className="atelier-collab-panel__section-title">Roles</h4>
          {rolesLoading && <span className="muted small">Loading...</span>}
        </div>
        {rolesError ? (
          <p className="error">Error: {rolesError}</p>
        ) : roles.length === 0 ? (
          <p className="muted">No roles configured.</p>
        ) : (
          <ul className="atelier-collab-panel__roles">
            {roles.map((role) => {
              const status = roleStatusById[role.role_id];
              const suggestions = suggestionsByRole[role.role_id] ?? [];
              return (
                <li key={role.role_id} className="atelier-collab-panel__role">
                  <div className="atelier-collab-panel__role-header">
                    <span className="atelier-collab-panel__role-name">{role.display_name}</span>
                    <button
                      type="button"
                      onClick={() => void generateForRole(role.role_id)}
                      disabled={!canOperate || applying}
                    >
                      Generate
                    </button>
                  </div>
                  {status && <p className="muted small">{status}</p>}
                  {suggestions.length > 0 && (
                    <ul className="atelier-collab-panel__suggestions">
                      {suggestions.map((s) => (
                        <li key={s.suggestion_id} className="atelier-collab-panel__suggestion">
                          <label className="atelier-collab-panel__suggestion-label">
                            <input
                              type="checkbox"
                              checked={Boolean(selectedIds[s.suggestion_id])}
                              onChange={(e) =>
                                setSelectedIds((prev) => ({
                                  ...prev,
                                  [s.suggestion_id]: e.target.checked,
                                }))
                              }
                              disabled={applying}
                            />
                            <span>{s.title}</span>
                          </label>
                        </li>
                      ))}
                    </ul>
                  )}
                </li>
              );
            })}
          </ul>
        )}
      </section>

      <div className="atelier-collab-panel__footer">
        <button
          type="button"
          onClick={() => void applySelected()}
          disabled={applying || allSuggestions.length === 0}
        >
          {applying ? "Applying..." : "Apply selected"}
        </button>
        {applyError && <p className="error">Error: {applyError}</p>}
      </div>
    </aside>
  );
}
