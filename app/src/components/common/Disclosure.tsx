import { useEffect, useId, useRef, useState, type ReactNode } from "react";

// Disclosure: a small, accessible, reusable collapsible section primitive.
//
// We deliberately use a real <button> summary row instead of native
// <details>/<summary> so we get: (1) reliable live count rendering in the
// summary, (2) full control over the default-open state across panes, and
// (3) stable test ids the Playwright visual matrix + CDP DOM-snapshot harness
// can target deterministically. The button gives Enter/Space toggle and a
// focus ring for free; the panel is a role="region" labelled by the button.

export interface DisclosureProps {
  /** Stable id used to derive deterministic test/stable ids and aria wiring. */
  id: string;
  /** Summary row title. */
  title: string;
  /** Optional muted count/status badge shown next to the title. */
  count?: string | number;
  /** Initial open state (default false = collapsed). */
  defaultOpen?: boolean;
  /**
   * When true, children are NOT rendered until the disclosure is first opened,
   * and stay mounted afterwards. Use for heavy panels that hold subscriptions
   * or polling loops (e.g. the live swarm board) so a collapsed section costs
   * nothing. Default false = children always mounted, just visually hidden.
   */
  lazy?: boolean;
  /** Collapsible body. */
  children: ReactNode;
  /**
   * An external "open me now" signal. Each time this value CHANGES to a new
   * non-undefined value, the disclosure force-opens (and marks itself
   * ever-opened so lazy children mount). The disclosure stays internally
   * uncontrolled afterwards — the operator can still collapse it — so this is a
   * one-shot programmatic open, not a controlled-open prop. Used by the board's
   * "Inspect terminal" affordance to reveal the off-main-window terminal drawer
   * on demand. Undefined = no external open driver (default).
   */
  openSignal?: number;
  "data-testid"?: string;
}

export function Disclosure({
  id,
  title,
  count,
  defaultOpen = false,
  lazy = false,
  children,
  openSignal,
  "data-testid": dataTestId,
}: DisclosureProps) {
  const [open, setOpen] = useState<boolean>(defaultOpen);
  // Once opened, stay opened-at-least-once so lazy children remain mounted even
  // after the operator collapses the section again. Tracked as state (not a ref)
  // so it is render-safe.
  const [everOpened, setEverOpened] = useState<boolean>(defaultOpen);

  // One-shot programmatic open: whenever the parent bumps `openSignal` to a new
  // value, force this disclosure open. We seed the ref with the initial mount
  // value so a statically-provided signal does not override `defaultOpen`; only
  // a *change* (the board's "Inspect terminal" click) triggers an open. The
  // disclosure remains operator-collapsible afterwards (uncontrolled), so this
  // only ever *reveals* the section; it never pins it open. Tracking the applied
  // signal in a ref (not state) avoids an extra render.
  const appliedSignal = useRef<number | undefined>(openSignal);
  useEffect(() => {
    if (openSignal === undefined || openSignal === appliedSignal.current) return;
    appliedSignal.current = openSignal;
    // Genuine external-driver -> React-state sync (an operator click in a SIBLING
    // component reveals this drawer). This is exactly the effect use the lint
    // rule's own guidance permits ("update React state when an external event
    // occurs"); it is not a derivable-during-render value, so disable the
    // cascading-render heuristic here with that rationale.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setOpen(true);
    setEverOpened(true);
  }, [openSignal]);
  const reactId = useId();
  const panelId = `disclosure-panel-${id}-${reactId}`;
  const buttonId = `disclosure-button-${id}-${reactId}`;

  return (
    <div
      className="disclosure"
      data-stable-id={`disclosure-${id}`}
      data-testid={dataTestId ?? `disclosure-${id}`}
      data-open={open ? "true" : "false"}
    >
      <button
        type="button"
        id={buttonId}
        className="disclosure__summary"
        aria-expanded={open}
        aria-controls={panelId}
        data-stable-id={`disclosure-${id}.toggle`}
        data-testid={`disclosure-${id}-toggle`}
        onClick={() => {
          setOpen((prev) => !prev);
          setEverOpened(true);
        }}
      >
        <span className="disclosure__chevron" aria-hidden="true">
          ▸
        </span>
        <span className="disclosure__title">{title}</span>
        {count != null ? (
          <span className="disclosure__count">{count}</span>
        ) : null}
      </button>
      <div
        role="region"
        id={panelId}
        aria-labelledby={buttonId}
        className="disclosure__panel"
        hidden={!open}
      >
        {lazy && !everOpened ? null : children}
      </div>
    </div>
  );
}
