import { useCallback, useEffect, useMemo, useState, type DragEvent } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import type { ModelCapabilities } from "../../lib/ipc/model_runtime";
import {
  LORA_STRENGTH_MAX,
  LORA_STRENGTH_MIN,
  loraList,
  loraMount,
  loraSwap,
  loraUnmount,
  type LoraDescriptor,
  type LoraStackEntry,
  type LoraStackItem,
} from "../../lib/ipc/lora";

// MT-091 owned-files contract listed
// `app/src/components/inference_lab/LoraStackComposer.svelte` and
// `app/src/lib/stores/lora_stack.ts`. App stack is React/TSX (same
// Svelte->TSX defect class as MT-098 / MT-102 / MT-105 / MT-124);
// behavior + acceptance criteria + red_team controls preserved.
//
// AC-INFER-LAB-UI-TOGGLES: hidden (display:none) when supportsLora=false.
// AC-INFER-LAB-UI-AB-COMPARE: A/B compare per-strength is deferred to a
// follow-up MT because there is no generation IPC surfaced yet. Recorded
// as residual_risk on MT-091. The strength slider UI surface lands here.
// All mutations route through the kernel_model_runtime_lora_* IPC, which
// preflights ModelRuntimeState.lora_command_binding via KernelActionCatalogV1
// before touching the live runtime (see app/src-tauri/src/commands/lora.rs).

type Props = {
  modelId: string;
  capabilities: ModelCapabilities | null;
};

type ComposerState =
  | { status: "loading" }
  | { status: "error"; message: string }
  | { status: "ready"; entries: ComposerEntry[] };

interface ComposerEntry {
  loraId: string;
  strength: number;
  mountedAtUtc: string;
  // Full descriptor when known client-side (populated from mount/swap
  // returns or from operator-supplied mount-from-disk form). Light entries
  // returned by loraList do not include a descriptor; rows without a
  // cached descriptor cannot participate in swap-based reorder or
  // Save-as-Work-Profile until the operator re-mounts them.
  descriptor: LoraDescriptor | null;
}

interface MountDraft {
  artifactPath: string;
  sha256: string;
  baseModelCompat: string;
  rank: string;
  targetModulesCsv: string;
  licenseTag: string;
  strength: string;
}

const DEFAULT_MOUNT_DRAFT: MountDraft = {
  artifactPath: "",
  sha256: "",
  baseModelCompat: "",
  rank: "8",
  targetModulesCsv: "q_proj,v_proj",
  licenseTag: "",
  strength: "1.0",
};

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  return "Unknown error";
}

// UUID v7 implementation aligned with the backend `LoraId::new_v7()`
// expectation. Browser crypto.randomUUID() emits v4, which the backend
// rejects via LoraId::try_from_uuid. Hand-rolled here because the app
// has no UUID v7 dependency and adding one for this MT is out of scope.
function newUuidV7(): string {
  const now = BigInt(Date.now());
  const tsHex = now.toString(16).padStart(12, "0");
  const rand = crypto.getRandomValues(new Uint8Array(10));
  // Set version 7 in the high nibble of byte 0 of the random block,
  // and the RFC 4122 variant (10xx) in the high two bits of byte 2.
  rand[0] = (rand[0] & 0x0f) | 0x70;
  rand[2] = (rand[2] & 0x3f) | 0x80;
  const hex = Array.from(rand, (b) => b.toString(16).padStart(2, "0")).join("");
  return [
    tsHex.slice(0, 8),
    tsHex.slice(8, 12),
    hex.slice(0, 4),
    hex.slice(4, 8),
    hex.slice(8, 20),
  ].join("-");
}

function toEntryFromLight(entry: LoraStackEntry, descriptor: LoraDescriptor | null): ComposerEntry {
  return {
    loraId: entry.loraId,
    strength: entry.strength,
    mountedAtUtc: entry.mountedAtUtc,
    descriptor,
  };
}

function parseRank(value: string): number {
  const trimmed = value.trim();
  if (!/^[0-9]+$/.test(trimmed)) {
    throw new Error("rank must be a non-negative integer");
  }
  const parsed = Number.parseInt(trimmed, 10);
  if (!Number.isInteger(parsed) || parsed <= 0) {
    throw new Error("rank must be a positive integer");
  }
  return parsed;
}

function parseTargetModules(csv: string): string[] {
  const items = csv
    .split(",")
    .map((part) => part.trim())
    .filter((part) => part.length > 0);
  if (items.length === 0) {
    throw new Error("at least one target module is required");
  }
  return items;
}

function parseSha256(value: string): string {
  const trimmed = value.trim().toLowerCase();
  if (!/^[0-9a-f]{64}$/.test(trimmed)) {
    throw new Error("sha256 must be 64 hex characters");
  }
  return trimmed;
}

function parseStrength(value: string): number {
  const parsed = Number.parseFloat(value);
  if (!Number.isFinite(parsed)) {
    throw new Error("strength must be a finite number");
  }
  if (parsed < LORA_STRENGTH_MIN || parsed > LORA_STRENGTH_MAX) {
    throw new Error(
      `strength must be within [${LORA_STRENGTH_MIN}, ${LORA_STRENGTH_MAX}]`,
    );
  }
  return parsed;
}

function buildDescriptorFromDraft(draft: MountDraft, loraId: string): LoraDescriptor {
  if (draft.artifactPath.trim().length === 0) {
    throw new Error("artifactPath must not be empty");
  }
  if (draft.baseModelCompat.trim().length === 0) {
    throw new Error("baseModelCompat must not be empty");
  }
  if (draft.licenseTag.trim().length === 0) {
    throw new Error("licenseTag must not be empty");
  }
  return {
    loraId,
    artifactPath: draft.artifactPath.trim(),
    sha256: parseSha256(draft.sha256),
    rank: parseRank(draft.rank),
    targetModules: parseTargetModules(draft.targetModulesCsv),
    baseModelCompat: draft.baseModelCompat.trim(),
    licenseTag: draft.licenseTag.trim(),
  };
}

function stackToItems(entries: ComposerEntry[]): LoraStackItem[] | null {
  // Returns null if any entry has no cached descriptor (which means
  // the full descriptor is not known client-side and cannot be sent
  // through the swap surface). Caller renders an explicit operator
  // warning so partial state never silently strips a LoRA.
  const out: LoraStackItem[] = [];
  for (const entry of entries) {
    if (!entry.descriptor) {
      return null;
    }
    out.push({ descriptor: entry.descriptor, strength: entry.strength });
  }
  return out;
}

export function LoraStackComposer({ modelId, capabilities }: Props) {
  const [state, setState] = useState<ComposerState>({ status: "loading" });
  const [mountDraft, setMountDraft] = useState<MountDraft | null>(null);
  const [mountError, setMountError] = useState<string | null>(null);
  const [pending, setPending] = useState<"swap" | "mount" | "unmount" | "save_profile" | null>(null);
  const [draggingIdx, setDraggingIdx] = useState<number | null>(null);
  const [profileNotice, setProfileNotice] = useState<string | null>(null);

  const supportsLora = capabilities?.supportsLora === true;

  const refresh = useCallback(async () => {
    if (!supportsLora) return;
    setState({ status: "loading" });
    try {
      const result = await loraList({ modelId });
      setState((prev) => {
        // Preserve known descriptors across refreshes (the IPC returns light
        // entries only).
        const prevCache = new Map<string, LoraDescriptor>();
        if (prev.status === "ready") {
          for (const entry of prev.entries) {
            if (entry.descriptor) prevCache.set(entry.loraId, entry.descriptor);
          }
        }
        const entries = result.activeStack.map((entry) =>
          toEntryFromLight(entry, prevCache.get(entry.loraId) ?? null),
        );
        return { status: "ready", entries };
      });
    } catch (error) {
      setState({ status: "error", message: errorMessage(error) });
    }
  }, [modelId, supportsLora]);

  useEffect(() => {
    let active = true;
    if (!supportsLora) {
      setState({ status: "ready", entries: [] });
      return () => {
        active = false;
      };
    }
    void refresh().then(() => {
      if (!active) return;
    });
    return () => {
      active = false;
    };
  }, [refresh, supportsLora]);

  const handleStrengthChange = useCallback(
    async (entry: ComposerEntry, nextStrength: number) => {
      if (state.status !== "ready") return;
      if (!entry.descriptor) {
        setState({
          status: "error",
          message: `Cannot adjust strength for ${entry.loraId}: descriptor not known client-side. Re-mount from disk to recover.`,
        });
        return;
      }
      const nextEntries = state.entries.map((row) =>
        row.loraId === entry.loraId ? { ...row, strength: nextStrength } : row,
      );
      const items = stackToItems(nextEntries);
      if (!items) {
        setState({
          status: "error",
          message:
            "Strength change requires all LoRAs in the stack to have a cached descriptor; re-mount any entry showing 'Descriptor unavailable'.",
        });
        return;
      }
      setPending("swap");
      try {
        const result = await loraSwap({
          modelId,
          stack: [],
          settings: { execPolicy: { loraStack: items } },
        });
        const cache = new Map<string, LoraDescriptor>();
        for (const item of items) cache.set(item.descriptor.loraId, item.descriptor);
        setState({
          status: "ready",
          entries: result.activeStack.map((row) =>
            toEntryFromLight(row, cache.get(row.loraId) ?? null),
          ),
        });
      } catch (error) {
        setState({ status: "error", message: errorMessage(error) });
      } finally {
        setPending(null);
      }
    },
    [modelId, state],
  );

  const handleUnmount = useCallback(
    async (loraId: string) => {
      if (state.status !== "ready") return;
      setPending("unmount");
      try {
        const result = await loraUnmount({ modelId, loraId });
        const prevCache = new Map<string, LoraDescriptor>();
        for (const entry of state.entries) {
          if (entry.descriptor) prevCache.set(entry.loraId, entry.descriptor);
        }
        setState({
          status: "ready",
          entries: result.activeStack.map((entry) =>
            toEntryFromLight(entry, prevCache.get(entry.loraId) ?? null),
          ),
        });
      } catch (error) {
        setState({ status: "error", message: errorMessage(error) });
      } finally {
        setPending(null);
      }
    },
    [modelId, state],
  );

  const handleOpenMountDialog = useCallback(async () => {
    setMountError(null);
    try {
      const picked = await open({
        multiple: false,
        directory: false,
        filters: [
          {
            name: "LoRA artifacts",
            extensions: ["safetensors", "bin", "gguf"],
          },
        ],
      });
      if (picked === null) return;
      const artifactPath = Array.isArray(picked) ? picked[0] : picked;
      setMountDraft({ ...DEFAULT_MOUNT_DRAFT, artifactPath: artifactPath });
    } catch (error) {
      setMountError(errorMessage(error));
    }
  }, []);

  const handleConfirmMount = useCallback(async () => {
    if (!mountDraft) return;
    setMountError(null);
    let descriptor: LoraDescriptor;
    let strength: number;
    try {
      descriptor = buildDescriptorFromDraft(mountDraft, newUuidV7());
      strength = parseStrength(mountDraft.strength);
    } catch (error) {
      setMountError(errorMessage(error));
      return;
    }
    setPending("mount");
    try {
      const result = await loraMount({
        modelId,
        descriptor,
        strength,
      });
      const prevCache = new Map<string, LoraDescriptor>();
      if (state.status === "ready") {
        for (const entry of state.entries) {
          if (entry.descriptor) prevCache.set(entry.loraId, entry.descriptor);
        }
      }
      prevCache.set(descriptor.loraId, descriptor);
      setState({
        status: "ready",
        entries: result.activeStack.map((entry) =>
          toEntryFromLight(entry, prevCache.get(entry.loraId) ?? null),
        ),
      });
      setMountDraft(null);
    } catch (error) {
      setMountError(errorMessage(error));
    } finally {
      setPending(null);
    }
  }, [modelId, mountDraft, state]);

  const handleCancelMount = useCallback(() => {
    setMountDraft(null);
    setMountError(null);
  }, []);

  const handleSaveAsWorkProfile = useCallback(async () => {
    if (state.status !== "ready") return;
    const items = stackToItems(state.entries);
    if (!items) {
      setState({
        status: "error",
        message:
          "Save as Work Profile requires all LoRAs to have a cached descriptor; re-mount any entry showing 'Descriptor unavailable'.",
      });
      return;
    }
    setProfileNotice(null);
    setPending("save_profile");
    try {
      // Save-as-Work-Profile sends the current stack through the swap
      // surface using settings.execPolicy.loraStack — the same channel the
      // backend treats as the Work Profile knob bridge (preferred over the
      // raw stack[] when present).
      const result = await loraSwap({
        modelId,
        stack: [],
        settings: { execPolicy: { loraStack: items } },
      });
      const cache = new Map<string, LoraDescriptor>();
      for (const item of items) cache.set(item.descriptor.loraId, item.descriptor);
      setState({
        status: "ready",
        entries: result.activeStack.map((entry) =>
          toEntryFromLight(entry, cache.get(entry.loraId) ?? null),
        ),
      });
      setProfileNotice(
        `Saved as Work Profile (event=${result.eventType}, entries=${items.length}).`,
      );
    } catch (error) {
      setState({ status: "error", message: errorMessage(error) });
    } finally {
      setPending(null);
    }
  }, [modelId, state]);

  const handleDragStart = useCallback((index: number) => {
    setDraggingIdx(index);
  }, []);

  const handleDragOver = useCallback((event: DragEvent<HTMLLIElement>) => {
    event.preventDefault();
    if (event.dataTransfer) {
      event.dataTransfer.dropEffect = "move";
    }
  }, []);

  const handleDrop = useCallback(
    async (event: DragEvent<HTMLLIElement>, targetIndex: number) => {
      event.preventDefault();
      const fromIndex = draggingIdx;
      setDraggingIdx(null);
      if (state.status !== "ready") return;
      if (fromIndex === null || fromIndex === targetIndex) return;
      const next = [...state.entries];
      const [moved] = next.splice(fromIndex, 1);
      next.splice(targetIndex, 0, moved);
      const items = stackToItems(next);
      if (!items) {
        setState({
          status: "error",
          message:
            "Reorder requires all LoRAs to have a cached descriptor; re-mount any entry showing 'Descriptor unavailable'.",
        });
        return;
      }
      setPending("swap");
      try {
        const result = await loraSwap({
          modelId,
          stack: [],
          settings: { execPolicy: { loraStack: items } },
        });
        const cache = new Map<string, LoraDescriptor>();
        for (const item of items) cache.set(item.descriptor.loraId, item.descriptor);
        setState({
          status: "ready",
          entries: result.activeStack.map((entry) =>
            toEntryFromLight(entry, cache.get(entry.loraId) ?? null),
          ),
        });
      } catch (error) {
        setState({ status: "error", message: errorMessage(error) });
      } finally {
        setPending(null);
      }
    },
    [draggingIdx, modelId, state],
  );

  const hasMissingDescriptor = useMemo(() => {
    if (state.status !== "ready") return false;
    return state.entries.some((entry) => entry.descriptor === null);
  }, [state]);

  if (!supportsLora) {
    return null;
  }

  return (
    <section
      className="inference-lab__panel inference-lab__lora-stack"
      data-testid="lora-stack-composer"
      aria-labelledby="lora-stack-composer-title"
    >
      <header className="inference-lab__panel-header">
        <h3 id="lora-stack-composer-title">LoRA Stack Composer</h3>
        <p className="muted" data-testid="lora-stack-composer.note">
          Ordered LoRA stack for the selected model. Strength changes,
          re-ordering, mount, unmount, and Save as Work Profile all dispatch
          through the kernel_model_runtime_lora_* IPC surface (MT-090), which
          preflights ModelRuntimeState.lora_command_binding through
          KernelActionCatalogV1 before touching the live runtime.
        </p>
      </header>

      {state.status === "loading" ? (
        <p data-testid="lora-stack-composer.loading">Loading LoRA stack…</p>
      ) : state.status === "error" ? (
        <p
          role="alert"
          className="inference-lab__error"
          data-testid="lora-stack-composer.error"
        >
          {state.message}
        </p>
      ) : state.entries.length === 0 ? (
        <p
          className="muted"
          data-testid="lora-stack-composer.empty"
        >
          No LoRAs mounted. Use Mount from disk… to add the first adapter.
        </p>
      ) : (
        <ul
          className="lora-stack-composer__list"
          data-testid="lora-stack-composer.list"
          aria-label="Mounted LoRA stack (drag to reorder)"
        >
          {state.entries.map((entry, index) => {
            const licenseTag = entry.descriptor?.licenseTag ?? "license unknown";
            const baseModel = entry.descriptor?.baseModelCompat ?? "base unknown";
            return (
              <li
                key={entry.loraId}
                className="lora-stack-composer__row"
                draggable
                onDragStart={() => handleDragStart(index)}
                onDragOver={handleDragOver}
                onDrop={(event) => void handleDrop(event, index)}
                data-testid={`lora-stack-composer.row.${entry.loraId}`}
              >
                <div className="lora-stack-composer__row-main">
                  <span
                    className="lora-stack-composer__handle"
                    aria-label="drag handle"
                    role="presentation"
                  >
                    ⠿
                  </span>
                  <div className="lora-stack-composer__identity">
                    <span
                      className="lora-stack-composer__id"
                      data-testid={`lora-stack-composer.row.${entry.loraId}.id`}
                    >
                      {entry.loraId}
                    </span>
                    <span
                      className="lora-stack-composer__license"
                      data-testid={`lora-stack-composer.row.${entry.loraId}.license`}
                    >
                      {licenseTag}
                    </span>
                    <span
                      className="lora-stack-composer__base"
                      data-testid={`lora-stack-composer.row.${entry.loraId}.base`}
                    >
                      {baseModel}
                    </span>
                    {entry.descriptor === null ? (
                      <span
                        className="lora-stack-composer__warn"
                        role="status"
                        data-testid={`lora-stack-composer.row.${entry.loraId}.missing-descriptor`}
                      >
                        Descriptor unavailable — re-mount from disk to enable
                        strength change, reorder, or Save as Work Profile.
                      </span>
                    ) : null}
                  </div>
                </div>
                <label className="lora-stack-composer__strength">
                  <span className="lora-stack-composer__strength-label">
                    strength {entry.strength.toFixed(2)}
                  </span>
                  <input
                    type="range"
                    min={LORA_STRENGTH_MIN}
                    max={LORA_STRENGTH_MAX}
                    step={0.05}
                    value={entry.strength}
                    onChange={(event) =>
                      void handleStrengthChange(entry, Number.parseFloat(event.target.value))
                    }
                    disabled={pending !== null || entry.descriptor === null}
                    data-testid={`lora-stack-composer.row.${entry.loraId}.strength`}
                  />
                </label>
                <button
                  type="button"
                  className="lora-stack-composer__unmount"
                  onClick={() => void handleUnmount(entry.loraId)}
                  disabled={pending !== null}
                  data-testid={`lora-stack-composer.row.${entry.loraId}.unmount`}
                >
                  Unmount
                </button>
              </li>
            );
          })}
        </ul>
      )}

      <div className="lora-stack-composer__actions">
        <button
          type="button"
          className="lora-stack-composer__mount"
          onClick={() => void handleOpenMountDialog()}
          disabled={pending !== null}
          data-testid="lora-stack-composer.mount-from-disk"
        >
          Mount from disk…
        </button>
        <button
          type="button"
          className="lora-stack-composer__save-profile"
          onClick={() => void handleSaveAsWorkProfile()}
          disabled={
            pending !== null ||
            state.status !== "ready" ||
            state.entries.length === 0 ||
            hasMissingDescriptor
          }
          data-testid="lora-stack-composer.save-profile"
        >
          Save as Work Profile
        </button>
      </div>

      {profileNotice ? (
        <p
          className="lora-stack-composer__notice"
          role="status"
          data-testid="lora-stack-composer.profile-notice"
        >
          {profileNotice}
        </p>
      ) : null}

      {mountDraft !== null ? (
        <div
          className="lora-stack-composer__mount-form"
          role="dialog"
          aria-modal="true"
          aria-labelledby="lora-stack-composer-mount-form-title"
          data-testid="lora-stack-composer.mount-form"
        >
          <h4 id="lora-stack-composer-mount-form-title">
            Mount LoRA from disk
          </h4>
          <p className="muted">
            Operator-supplied descriptor metadata. The backend validates each
            field against the LoraDescriptor contract and rejects the mount
            before touching the runtime if any field is malformed.
          </p>
          <label>
            <span>Artifact path</span>
            <input
              type="text"
              value={mountDraft.artifactPath}
              onChange={(event) =>
                setMountDraft((draft) =>
                  draft
                    ? { ...draft, artifactPath: event.target.value }
                    : draft,
                )
              }
              data-testid="lora-stack-composer.mount-form.artifact-path"
            />
          </label>
          <label>
            <span>sha256 (hex)</span>
            <input
              type="text"
              value={mountDraft.sha256}
              onChange={(event) =>
                setMountDraft((draft) =>
                  draft ? { ...draft, sha256: event.target.value } : draft,
                )
              }
              data-testid="lora-stack-composer.mount-form.sha256"
            />
          </label>
          <label>
            <span>Base model compat</span>
            <input
              type="text"
              value={mountDraft.baseModelCompat}
              onChange={(event) =>
                setMountDraft((draft) =>
                  draft
                    ? { ...draft, baseModelCompat: event.target.value }
                    : draft,
                )
              }
              data-testid="lora-stack-composer.mount-form.base-model-compat"
            />
          </label>
          <label>
            <span>Rank</span>
            <input
              type="text"
              value={mountDraft.rank}
              onChange={(event) =>
                setMountDraft((draft) =>
                  draft ? { ...draft, rank: event.target.value } : draft,
                )
              }
              data-testid="lora-stack-composer.mount-form.rank"
            />
          </label>
          <label>
            <span>Target modules (comma-separated)</span>
            <input
              type="text"
              value={mountDraft.targetModulesCsv}
              onChange={(event) =>
                setMountDraft((draft) =>
                  draft
                    ? { ...draft, targetModulesCsv: event.target.value }
                    : draft,
                )
              }
              data-testid="lora-stack-composer.mount-form.target-modules"
            />
          </label>
          <label>
            <span>License tag</span>
            <input
              type="text"
              value={mountDraft.licenseTag}
              onChange={(event) =>
                setMountDraft((draft) =>
                  draft
                    ? { ...draft, licenseTag: event.target.value }
                    : draft,
                )
              }
              data-testid="lora-stack-composer.mount-form.license-tag"
            />
          </label>
          <label>
            <span>Strength ({LORA_STRENGTH_MIN}..{LORA_STRENGTH_MAX})</span>
            <input
              type="text"
              value={mountDraft.strength}
              onChange={(event) =>
                setMountDraft((draft) =>
                  draft ? { ...draft, strength: event.target.value } : draft,
                )
              }
              data-testid="lora-stack-composer.mount-form.strength"
            />
          </label>
          {mountError ? (
            <p
              role="alert"
              className="inference-lab__error"
              data-testid="lora-stack-composer.mount-form.error"
            >
              {mountError}
            </p>
          ) : null}
          <div className="lora-stack-composer__mount-form-actions">
            <button
              type="button"
              onClick={() => void handleConfirmMount()}
              disabled={pending !== null}
              data-testid="lora-stack-composer.mount-form.confirm"
            >
              Mount
            </button>
            <button
              type="button"
              onClick={handleCancelMount}
              disabled={pending !== null}
              data-testid="lora-stack-composer.mount-form.cancel"
            >
              Cancel
            </button>
          </div>
        </div>
      ) : null}

      {mountError && mountDraft === null ? (
        <p
          role="alert"
          className="inference-lab__error"
          data-testid="lora-stack-composer.mount-error"
        >
          {mountError}
        </p>
      ) : null}
    </section>
  );
}
