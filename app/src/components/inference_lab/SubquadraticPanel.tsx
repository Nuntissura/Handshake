import { useState } from "react";
import type { ModelCapabilities } from "../../lib/ipc/model_runtime";
import {
  SUBQUAD_VARIANT_LABELS,
  type SubquadVariant,
} from "../../lib/ipc/subquadratic";
import { StateVectorControls } from "./StateVectorControls";

// MT-113 — INF-9 Subquadratic UI panel.
//
// Owned files contract listed Svelte sources. App stack is React/TSX
// (same Svelte->TSX defect class as MT-091/094/098/102/105/124/110/112).
// Behavior + AC + red_team controls preserved.
//
// AC-INFER-LAB-UI-TOGGLES: hidden (return null) when
// capabilities.supportsSubquadratic === false. Variant badge prominent;
// state-vector controls inline (StateVectorControls). Operator can opt
// out via a Work Profile flag (settings.ui.show_subquadratic_panel)
// without losing model state — the panel just hides itself.
//
// Variant detection: LoadedModelRuntime does not yet carry the SSM
// variant; the panel accepts an optional `variant` prop and falls back
// to "Subquadratic (variant detection pending)" when undefined. The
// variant-aware loader is a follow-on MT (the contract narrative
// references `subquadratic-specific config (e.g., n_layer, d_state
// read-only display from model config)` which lands alongside the
// loader update).

type Props = {
  modelId: string;
  capabilities: ModelCapabilities | null;
  /** SSM variant for the badge. Optional until the loader emits it. */
  variant?: SubquadVariant;
};

const WORK_PROFILE_OPT_OUT_KEY = "settings.ui.show_subquadratic_panel";

function readWorkProfileOptOut(): boolean {
  // Work Profile flag: settings.ui.show_subquadratic_panel — operator
  // sets this to "false" if they're not yet using SSMs and want the
  // panel hidden even on capable models. We read from localStorage as
  // the interim store (the Work Profile IPC bridge lands in a separate
  // MT). Default: panel visible (false means opted-out).
  if (typeof window === "undefined") return false;
  try {
    const raw = window.localStorage.getItem(WORK_PROFILE_OPT_OUT_KEY);
    if (raw === null) return false;
    return raw.trim().toLowerCase() === "false";
  } catch {
    return false;
  }
}

export function SubquadraticPanel({ modelId, capabilities, variant }: Props) {
  const supportsSubquadratic = capabilities?.supportsSubquadratic === true;
  const [optedOut, setOptedOut] = useState<boolean>(() => readWorkProfileOptOut());

  if (!supportsSubquadratic) return null;

  const variantLabel = variant
    ? SUBQUAD_VARIANT_LABELS[variant]
    : "Subquadratic (variant detection pending)";

  const toggleOptOut = () => {
    const next = !optedOut;
    setOptedOut(next);
    try {
      if (typeof window !== "undefined") {
        window.localStorage.setItem(
          WORK_PROFILE_OPT_OUT_KEY,
          next ? "false" : "true",
        );
      }
    } catch {
      // localStorage may be unavailable (private browsing); the panel
      // still flips state for this session.
    }
  };

  return (
    <section
      className="inference-lab__panel inference-lab__subquadratic"
      data-testid="subquadratic-panel"
      aria-labelledby="subquadratic-panel-title"
    >
      <header className="inference-lab__panel-header">
        <div className="subquadratic-panel__title-row">
          <h3 id="subquadratic-panel-title">Subquadratic state vectors</h3>
          <span
            className="subquadratic-panel__variant-badge"
            data-testid="subquadratic-panel.variant-badge"
          >
            {variantLabel}
          </span>
        </div>
        <p className="muted" data-testid="subquadratic-panel.note">
          State-vector cache for SSM/RWKV models. Commit a prefix's SSM
          state, restore it later in the session, or evict everything.
          Disk persistence (cross-session restore) lands in MT-117.
        </p>
      </header>

      <div className="subquadratic-panel__opt-out">
        <label>
          <input
            type="checkbox"
            checked={optedOut}
            onChange={toggleOptOut}
            data-testid="subquadratic-panel.opt-out-toggle"
          />
          <span>
            Hide this panel (Work Profile flag {WORK_PROFILE_OPT_OUT_KEY})
          </span>
        </label>
      </div>

      {optedOut ? (
        <p
          className="muted"
          data-testid="subquadratic-panel.opted-out-note"
        >
          Subquadratic panel hidden by Work Profile preference. Uncheck the
          opt-out box above to re-enable.
        </p>
      ) : (
        <StateVectorControls modelId={modelId} optedOut={optedOut} />
      )}
    </section>
  );
}
