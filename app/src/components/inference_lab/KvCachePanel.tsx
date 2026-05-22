import { useCallback, useEffect, useRef, useState } from "react";
import type { ModelCapabilities } from "../../lib/ipc/model_runtime";
import {
  kvEvictAll,
  kvOccupancy,
  kvSetQuantization,
  quantOptionsFor,
  type KvCacheStats,
  type KvQuantSupport,
} from "../../lib/ipc/kv_cache";

// MT-094 owned-files contract listed
// `app/src/components/inference_lab/KvCachePanel.svelte` and
// `app/src/lib/stores/kv_cache.ts`. App stack is React/TSX (same
// Svelte->TSX defect class as MT-091/098/102/105/124); behavior +
// acceptance criteria + red_team controls preserved.
//
// AC-INFER-LAB-UI-TOGGLES: hidden (display:none) when
// `supportsKvQuantization === "none"` AND `supportsKvPrefixCache === false`.
// When only one of the two capabilities is declared, the panel renders
// the supported half (e.g., the prefix TTL slider remains visible on a
// prefix-only adapter even if quant is None).
// AC-INFER-LAB-UI-AB-COMPARE: A/B compare is deferred to a follow-up MT
// (no generation IPC exposed yet — same residual_risk as MT-091).
// AC-MODEL-RUNTIME-CONTROL-PANEL: live occupancy bar + hit-rate badge
// satisfy the panel field set for this MT.
//
// Mutations route through the kernel_model_runtime_kv_* Tauri commands
// (MT-093), which preflight ModelRuntimeState.kv_cache_command_binding
// via KernelActionCatalogV1.

type Props = {
  modelId: string;
  capabilities: ModelCapabilities | null;
};

type PanelState =
  | { status: "loading" }
  | { status: "error"; message: string }
  | { status: "ready"; occupancy: KvCacheStats };

const OCCUPANCY_POLL_MS = 2000;
const PREFIX_TTL_MIN = 30;
const PREFIX_TTL_MAX = 3600;

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  return "Unknown error";
}

function formatBytes(bytes: number): string {
  if (bytes <= 0) return "0 B";
  const units = ["B", "KiB", "MiB", "GiB"];
  let value = bytes;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }
  return `${value.toFixed(value >= 100 ? 0 : 1)} ${units[unitIndex]}`;
}

function hitRatePercent(occupancy: KvCacheStats): number {
  const total = occupancy.prefixCacheHitCount + occupancy.prefixCacheMissCount;
  if (total === 0) return 0;
  return Math.round((occupancy.prefixCacheHitCount / total) * 100);
}

export function KvCachePanel({ modelId, capabilities }: Props) {
  const supportsQuant = capabilities?.supportsKvQuantization ?? "none";
  const supportsPrefix = capabilities?.supportsKvPrefixCache === true;
  const panelHidden = supportsQuant === "none" && !supportsPrefix;

  const [state, setState] = useState<PanelState>({ status: "loading" });
  const [currentQuant, setCurrentQuant] = useState<KvQuantSupport>("none");
  const [pendingQuant, setPendingQuant] = useState<KvQuantSupport | null>(null);
  const [prefixTtlSeconds, setPrefixTtlSeconds] = useState<number>(300);
  const [confirmEvict, setConfirmEvict] = useState<boolean>(false);
  const [pending, setPending] = useState<"quant" | "evict" | null>(null);
  const cancelRef = useRef(false);

  const refresh = useCallback(async (): Promise<void> => {
    try {
      const result = await kvOccupancy({ modelId });
      if (cancelRef.current) return;
      setState({ status: "ready", occupancy: result.occupancy });
      setCurrentQuant(result.occupancy.quantLevelCurrent);
    } catch (error) {
      if (cancelRef.current) return;
      setState({ status: "error", message: errorMessage(error) });
    }
  }, [modelId]);

  useEffect(() => {
    if (panelHidden) {
      setState({
        status: "ready",
        occupancy: {
          bytesUsed: 0,
          bytesCapacity: 0,
          prefixCacheEntries: 0,
          prefixCacheHitCount: 0,
          prefixCacheMissCount: 0,
          quantLevelCurrent: "none",
        },
      });
      return;
    }

    cancelRef.current = false;
    void refresh();
    // Live occupancy polling capped at OCCUPANCY_POLL_MS per the
    // red_team minimum control "polling does not flood IPC".
    const interval = window.setInterval(() => {
      void refresh();
    }, OCCUPANCY_POLL_MS);
    return () => {
      cancelRef.current = true;
      window.clearInterval(interval);
    };
  }, [refresh, panelHidden]);

  const handleQuantChange = useCallback(
    async (next: KvQuantSupport) => {
      if (next === currentQuant) return;
      setPendingQuant(next);
      setPending("quant");
      try {
        const result = await kvSetQuantization({
          modelId,
          settings: {
            execPolicy: {
              quantization: next,
              prefixCacheTtlSeconds: prefixTtlSeconds,
            },
          },
        });
        setCurrentQuant(result.currentQuantization);
        setPendingQuant(null);
        await refresh();
      } catch (error) {
        setState({ status: "error", message: errorMessage(error) });
        setPendingQuant(null);
      } finally {
        setPending(null);
      }
    },
    [currentQuant, modelId, prefixTtlSeconds, refresh],
  );

  const handleEvictAllRequest = useCallback(() => {
    setConfirmEvict(true);
  }, []);

  const handleEvictAllConfirm = useCallback(async () => {
    setConfirmEvict(false);
    setPending("evict");
    try {
      await kvEvictAll({ modelId });
      await refresh();
    } catch (error) {
      setState({ status: "error", message: errorMessage(error) });
    } finally {
      setPending(null);
    }
  }, [modelId, refresh]);

  const handleEvictAllCancel = useCallback(() => {
    setConfirmEvict(false);
  }, []);

  if (panelHidden) return null;

  const quantOptions = quantOptionsFor(supportsQuant);
  const showQuantPicker = quantOptions.length > 0;
  const showPrefixControls = supportsPrefix;

  return (
    <section
      className="inference-lab__panel inference-lab__kv-cache"
      data-testid="kv-cache-panel"
      aria-labelledby="kv-cache-panel-title"
    >
      <header className="inference-lab__panel-header">
        <h3 id="kv-cache-panel-title">KV Cache</h3>
        <p className="muted" data-testid="kv-cache-panel.note">
          Quant level, prefix-cache TTL, live occupancy, and Evict-all all
          route through kernel_model_runtime_kv_* (MT-093), which preflights
          via KernelActionCatalogV1 before touching the live runtime.
        </p>
      </header>

      {state.status === "loading" ? (
        <p data-testid="kv-cache-panel.loading">Loading KV cache state…</p>
      ) : state.status === "error" ? (
        <p
          role="alert"
          className="inference-lab__error"
          data-testid="kv-cache-panel.error"
        >
          {state.message}
        </p>
      ) : (
        <div className="kv-cache-panel__body">
          {showQuantPicker ? (
            <label className="kv-cache-panel__quant">
              <span>Quantization</span>
              <select
                value={pendingQuant ?? currentQuant}
                onChange={(event) =>
                  void handleQuantChange(event.target.value as KvQuantSupport)
                }
                disabled={pending !== null}
                data-testid="kv-cache-panel.quant-picker"
              >
                {quantOptions.map((opt) => (
                  <option key={opt} value={opt}>
                    {opt}
                  </option>
                ))}
              </select>
            </label>
          ) : null}

          {showPrefixControls ? (
            <>
              <label className="kv-cache-panel__ttl">
                <span>
                  Prefix cache TTL — {prefixTtlSeconds}s ({PREFIX_TTL_MIN}–
                  {PREFIX_TTL_MAX})
                </span>
                <input
                  type="range"
                  min={PREFIX_TTL_MIN}
                  max={PREFIX_TTL_MAX}
                  step={30}
                  value={prefixTtlSeconds}
                  onChange={(event) =>
                    setPrefixTtlSeconds(Number.parseInt(event.target.value, 10))
                  }
                  disabled={pending !== null}
                  data-testid="kv-cache-panel.prefix-ttl"
                />
              </label>
              <div
                className="kv-cache-panel__occupancy"
                data-testid="kv-cache-panel.occupancy"
              >
                <span data-testid="kv-cache-panel.occupancy.bytes">
                  Used: {formatBytes(state.occupancy.bytesUsed)} /{" "}
                  {formatBytes(state.occupancy.bytesCapacity)}
                </span>
                <span data-testid="kv-cache-panel.occupancy.entries">
                  Entries: {state.occupancy.prefixCacheEntries}
                </span>
                <span data-testid="kv-cache-panel.occupancy.hit-rate">
                  Hit rate: {hitRatePercent(state.occupancy)}% (
                  {state.occupancy.prefixCacheHitCount}H /{" "}
                  {state.occupancy.prefixCacheMissCount}M)
                </span>
              </div>
              <button
                type="button"
                onClick={handleEvictAllRequest}
                disabled={pending !== null}
                data-testid="kv-cache-panel.evict-all"
              >
                Evict all prefixes…
              </button>
            </>
          ) : null}
        </div>
      )}

      {confirmEvict ? (
        <div
          className="kv-cache-panel__confirm-evict"
          role="dialog"
          aria-modal="true"
          aria-labelledby="kv-cache-panel-confirm-title"
          data-testid="kv-cache-panel.confirm-evict"
        >
          <h4 id="kv-cache-panel-confirm-title">Evict all prefixes?</h4>
          <p className="muted">
            This wipes every committed KV prefix for the selected model.
            Generation in flight will rebuild its prefix from scratch.
          </p>
          <div className="kv-cache-panel__confirm-evict-actions">
            <button
              type="button"
              onClick={() => void handleEvictAllConfirm()}
              data-testid="kv-cache-panel.confirm-evict.confirm"
            >
              Evict all
            </button>
            <button
              type="button"
              onClick={handleEvictAllCancel}
              data-testid="kv-cache-panel.confirm-evict.cancel"
            >
              Cancel
            </button>
          </div>
        </div>
      ) : null}
    </section>
  );
}
